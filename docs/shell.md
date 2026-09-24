# Kernel Shell

The kernel shell is the interactive command-line interface running as task slot 3 (`keyboard_loop` in `src/input/keyboard.rs`). It is started during `init::process::setup_processes` and runs for the lifetime of the kernel (or till killed).

The purpose of having the kernel shell present is to provide a diagnostic command-line interface (CLI) when the system's state needs to be looked at. It is not intended to use the kernel shell as the main system shell: usage of the `SH.ELF` shell or a remote `TNT.ELF` shell instead is encouraged (see the [Application Suite](sdk/apps.md) page for more info).

**The kernel shell should be considered to be the system rescue shell primarily**. 

---

## Shell Loop

`keyboard_loop()` runs in an infinite loop:

+ Print the prompt via `config::get_prompt()`
```
user@host:path >
```
+ Read characters from `SCANCODE_BUF` (Set 1 scancodes, translated to ASCII). Buffer capacity: 128 bytes.
+ Special keys handled inline:
      - **Enter** — dispatch the accumulated input to `cmd::handle(input)`, then clear the buffer.
      - **Backspace** — remove the last character from the buffer and erase it from the display.
      - **Tab** — attempt FAT12 prefix completion (see below).
      - **Ctrl+L** — clear the screen (`clear_screen!()`).

+ Printable characters are echoed and appended to the input buffer.

The shell never exits. Starting a foreground process (`fg`) records the shell as the child's *waiter* and parks it (`Idle`); it is woken when that child exits, is killed or crashes — not when some unrelated background process ends. See [Scheduler](multitasking/scheduler.md#foreground-processes-and-waiters).

### Tab Completion

When Tab is pressed, the current input is treated as a filename prefix. The shell scans the current FAT12 directory (via `for_each_entry`) for entries whose 8.3 name starts with the uppercased prefix. If exactly one match exists, the buffer is replaced with the lowercased match name. Multiple matches are printed but the buffer is left unchanged.

---

## Command Dispatch

`cmd::handle(input: &[u8])`:

1. `split_cmd(input)` splits on the first space → `(cmd_name, args)`.
2. Linear search through `COMMANDS` for an exact name match.
3. If found: calls `cmd.function(args)`.
4. If not found and input is non-empty: prints `Unknown command: <name>`.

`split_cmd` is also used by individual command implementations to parse their own arguments.

---

## Built-in Commands

Commands marked **hidden** do not appear in `help` output.

### `beep`

Plays the built-in MIDI melody via the PC speaker (`audio::midi::play_melody`), then stops the speaker.

### `bg <binary>`

Loads and runs an ELF binary in the **background** (shell remains interactive). The binary name must be ≤ 8 characters; `.elf` is appended when no extension is given. The binary is looked up in the current working directory first (the FAT12 floppy, or the ISO when the cwd is under `/mnt/iso`), then in `/mnt/iso/bin`, so the programs shipped on the ISO image can be started from anywhere.

```
bg eth
bg garn --config /mnt/fat/GARN/GARN.CFG
```

### `fg <binary>`

Same as `bg` but runs in the **foreground** — the shell blocks until the process exits.

```
fg sh
```

### `cd <path>`

Changes the current working directory. Updates `SYSTEM_CONFIG.path` and `path_cluster`.

The argument is joined onto the current path and `.` and `..` are collapsed
before anything is looked up, so a relative name is always resolved on the
filesystem the working directory sits on.

- `cd /` — reset to VFS root.
- `cd ..` — go to parent; at the root it stays at the root.
- `cd <name>` — relative to the current directory, on FAT12 or ISO9660.
- `cd /mnt/fat/<path>` — absolute FAT12 path.
- `cd /mnt/iso/<path>` — absolute ISO9660 path (validates directory exists).

Multi-component paths (`foo/bar`, `../bar`) are supported.

The working directory is kept in a 32-byte field that userland also reads back
through `sysinfo`, so a path longer than that is refused with `cd: path too
long` instead of being stored truncated.

### `cls`

Clears the screen (fills framebuffer/VGA buffer with black).

### `debug` *(hidden)*

Dumps the in-memory debug ring buffer to the display and attempts to write it to `DEBUG.TXT` on FAT12.

### `dir [path]`

Lists directory contents. Without an argument: lists the current working directory. With a path argument: lists that directory (absolute or relative, FAT12 or ISO9660).

A path argument is resolved exactly as `cd` resolves one: joined onto the
working directory when it is relative, with `.` and `..` collapsed first.

Output format: one entry per line, directories have a trailing `/`.

```
dir
dir /mnt/iso
dir GFX
dir games
dir ../bin
```

### `echo <text>`

Prints the argument string followed by a newline.

```
echo hello world
```

### `fsck`

Runs the FAT12 filesystem check (`fs::fat12::check::run_check`). Prints a report with error count, orphaned clusters, cross-linked clusters, and invalid entries.

### `help`

Lists all non-hidden commands with their one-line descriptions.

### `heap`

Prints the userland heap (4 MiB at `0xC00000`): used and free bytes, the largest free block, block counts, and the bytes held per scheduler slot (`-` for untagged blocks). A slot with bytes here but no entry in `ts` is a leak that outlived its process.

When the heap lock is busy the command prints which slot holds it instead — a heap that stays busy is one whose holder died holding it.

```
Userland heap (4 MiB at 0xC00000)
  used         20480
  free         4173800
  largest free 4173800
  blocks       3 (1 free)
  slot 5   16384
  slot 6   4096
```

### `hlt`

Initiates system shutdown. Prints a shutdown message with a short delay, then calls `acpi::shutdown::shutdown()`. Falls back to a halt loop if ACPI shutdown is unavailable.

### `kill <pid>`

Kills the process with the given PID — the number `ts` prints — via `task::scheduler::kill_by_id`. PIDs are not slots: slots are reused, PIDs are not. Its page tables and heap blocks are released and a launcher waiting on it is woken. Prints `kill: no such PID` if nothing live has that PID.

```
kill 3
```

### `meminfo`

Prints the total usable RAM reported by the Multiboot2 memory map at boot.

```
Total RAM: 2047 MiB (2146959360 bytes)
```

### `mkdir <dirname>`

Creates a subdirectory in the current FAT12 directory. Name is uppercased to 8.3 format. Maximum name length: 11 bytes.

```
mkdir MYDIR
```

### `mount`

Lists all active VFS mount table entries. Output: one line per mount, format `<path> (<fstype>)`.

```
/ (rootfs)
/mnt/fat (fat12)
/mnt/iso (iso9660)
```

### `mv <old> <new>`

Renames a file in the current FAT12 directory. Both names are converted to 8.3 format. Does not change the file's data or cluster chain.

```
mv FOO.TXT BAR.TXT
```

### `read <filename>`

Prints the contents of a file. Supports both FAT12 (relative or absolute) and ISO9660 paths. Reads up to 4096 bytes.

```
read HELLO.TXT
read /mnt/fat/GARN/INDEX.HTM
read /mnt/iso/readme.txt
```

### `reset`

Force resets the VGA video mode to `0x03` (text mode).

### `rm <filename>`

Deletes a file from the current FAT12 directory: its cluster chain is returned to the FAT, then the directory entry is marked `0xE5` (deleted). Directories are not matched.

```
rm OLD.TXT
```

### `run <binary>` *(hidden)*

Alias for `fg` with a slightly different length limit (12 bytes). Loads and runs an ELF binary in the foreground.

### `time`

Reads the real-time clock (RTC/CMOS) and prints the current UTC time and date.

```
RTC Time: 14:32:07
RTC Date: 08/05/2026
```

### `ts`

Lists all currently running tasks via `task::scheduler::list_processes`: one line per occupied scheduler slot with the slot, PID, name, mode (`K`/`U`) and status. Crashed processes stay listed until their slot is needed.

```
SLOT PID NAME                M STATUS
0    0   kmain               K (Running)
2    2   kclock              K (Ready)
3    3   kshell              K (Ready)
4    7   eth                 U (Blocked)
```

### `uptime` *(hidden)*

Prints system uptime in hours, minutes, and seconds using the PIT tick counter.

```
Uptime: 0 hours 3 minutes 41 seconds
```

### `ver`

Prints the kernel version string.

```
Version: 0.11.0
```

---

## Prompt Format

The prompt is assembled by `config::get_prompt()` from `SYSTEM_CONFIG`:

```
user@host:path >
```

Example: `root@rourex:/ > ` or `root@rourex:/mnt/fat/GFX > `.

Falls back to `$ ` if the config lock is contended.

---

## ELF Execution

`bg` and `fg` both delegate to `input::elf::run_elf(filename, args, mode)`:

1. Asks the scheduler which slot the program will occupy (`next_free_slot`); fails with `no free process slot` when all ten are held by live processes.
2. Finds the ELF file — current working directory first (FAT12, or ISO9660 when the cwd is under `/mnt/iso`), then `/mnt/iso/bin` — and stages it at a per-slot scratch address.
3. Copies the `PT_LOAD` segments into the slot's private 2 MiB physical frame and builds a page table that maps it at `0x600_000`.
4. Pushes the argv frame onto the slot's initial user stack and creates the scheduler task at the ELF entry point.
5. `Foreground`: the launcher (the shell, or `init_rc`) is recorded as the child's waiter and parked until the child ends.
6. `Background`: returns immediately; the shell stays interactive.

Userland processes communicate with the kernel via interrupt `0x7F` (syscall gate). See [Syscall specification](abi/syscall_specification.md) for the full syscall interface, and the [SDK](sdk/index.md) for building programs.
