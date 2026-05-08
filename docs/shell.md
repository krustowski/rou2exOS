# Kernel Shell

The kernel shell is the interactive command-line interface running as task slot 3 (`keyboard_loop` in `src/input/keyboard.rs`). It is started during `init::process::setup_processes` and runs for the lifetime of the kernel.

---

## Shell Loop

`keyboard_loop()` runs in an infinite loop:

1. Print the prompt via `config::get_prompt()` → `user@host:path > `.
2. Read characters from `SCANCODE_BUF` (Set 1 scancodes, translated to ASCII). Buffer capacity: 128 bytes.
3. Special keys handled inline:
   - **Enter** — dispatch the accumulated input to `cmd::handle(input)`, then clear the buffer.
   - **Backspace** — remove the last character from the buffer and erase it from the display.
   - **Tab** — attempt FAT12 prefix completion (see below).
   - **Ctrl+L** — clear the screen (`clear_screen!()`).
4. Printable characters are echoed and appended to the input buffer.

The shell never exits; a foreground process (`fg`) causes the shell task to yield (via `scheduler::idle`) until the child signals completion via syscall `0x00`.

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

Loads and runs an ELF binary in the **background** (shell remains interactive). The binary name must be ≤ 8 characters. The ELF is loaded from the FAT12 filesystem using the current working directory.

```
bg ETH
bg GARN --config /mnt/fat/GARN/GARN.CFG
```

### `fg <binary>`

Same as `bg` but runs in the **foreground** — the shell blocks until the process exits.

```
fg SH
```

### `cd <path>`

Changes the current working directory. Updates `SYSTEM_CONFIG.path` and `path_cluster`.

- `cd /` — reset to VFS root.
- `cd ..` — go to parent (FAT12: follows the `.` FAT entry; ISO9660: trims the path string).
- `cd <name>` — relative component; appended to current path.
- `cd /mnt/fat/<path>` — absolute FAT12 path.
- `cd /mnt/iso/<path>` — absolute ISO9660 path (validates directory exists).

Multi-component paths (`foo/bar`) are supported.

### `cls`

Clears the screen (fills framebuffer/VGA buffer with black).

### `debug` *(hidden)*

Dumps the in-memory debug ring buffer to the display and attempts to write it to `DEBUG.TXT` on FAT12.

### `dir [path]`

Lists directory contents. Without an argument: lists the current working directory. With a path argument: lists that directory (absolute or relative, FAT12 or ISO9660).

Output format: one entry per line, directories have a trailing `/`.

```
dir
dir /mnt/iso
dir GFX
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

### `hlt`

Initiates system shutdown. Prints a shutdown message with a short delay, then calls `acpi::shutdown::shutdown()`. Falls back to a halt loop if ACPI shutdown is unavailable.

### `kill <pid>`

Sends a kill signal to the process with the given numeric PID via `task::scheduler::kill(pid)`.

```
kill 3
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

### `rm <filename>`

Deletes a file from the current FAT12 directory. Marks the directory entry as `0xE5` (deleted). Clusters are not immediately freed; they are reclaimed on the next `write_file` call to the same name.

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

Lists all currently running tasks via `task::scheduler::list_processes`. Output includes PID, state, and name for each scheduler slot.

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

1. Finds the ELF file in the current FAT12 directory.
2. Loads it into the userland heap region (`0xC00_000–0xFFF_FFF`).
3. Creates a new scheduler task entry pointing at the ELF entry point.
4. `Foreground`: the shell task yields (`scheduler::idle`) until the child exits.
5. `Background`: returns immediately; the shell stays interactive.

Userland processes communicate with the kernel via interrupt `0x7F` (syscall gate). See [docs/ABI/syscall_specification.md](ABI/syscall_specification.md) for the full syscall interface.
