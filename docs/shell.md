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

Nothing is completed while the working directory is on no FAT12 volume: `/`, `/mnt`, the ISO or the archive.

---

## Command Dispatch

`cmd::handle(input: &[u8])`:

1. `split_cmd(input)` splits on the first space → `(cmd_name, args)`.
2. Linear search through `COMMANDS` for an exact name match.
3. If found: calls `cmd.function(args)`.
4. If not found and input is non-empty: prints `Unknown command: <name>`.

`split_cmd` is also used by individual command implementations to parse their own arguments.

---

## The Root Directory

`/` is the root of the VFS mount table, not a disk. It and `/mnt` hold nothing but the way to the mounted filesystems: `dir` lists what is mounted below them, `cd` moves through them, and the commands that work on files (`read`, `rm`, `write`, `mkdir`, `mv`) refuse to work there. The shell starts at `/`, so the first `dir` after boot shows `mnt`. Every file lives below a mount:

| Path | Filesystem | Mounted |
|------|------------|---------|
| `/mnt/fat` | the FAT12 floppy | only when a FAT12 floppy was found at boot |
| `/mnt/tmp` | the RAM disk, FAT16, a sixteenth of the RAM | always (empty at every boot) |
| `/mnt/iso` | the CD, ISO9660, read-only | when a CD is found |
| `/mnt/tar` | the boot medium archive, read-only | when GRUB loaded one |

A bare path under `/` used to name the floppy's root: `dir` at `/` listed the floppy, and `write` there wrote to it. Booted from a USB stick, with no floppy, that was a directory that could only answer with disk errors. User programs still see it that way; see [Filesystem syscalls](abi/syscalls/filesystem.md).

---

## Built-in Commands

Commands marked **hidden** do not appear in `help` output.

### `beep`

Plays the built-in MIDI melody via the PC speaker (`audio::midi::play_melody`), then stops the speaker.

### `bg <binary>`

Loads and runs an ELF binary in the **background** (shell remains interactive). The binary name must be ≤ 8 characters; `.elf` is appended when no extension is given. The binary is looked up in the current working directory first (when that is on a FAT12 volume, the ISO or the archive; `/` and `/mnt` hold no files), then in `/mnt/tar/bin` and `/mnt/iso/bin`, so the programs shipped on the boot medium can be started from anywhere.

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

- `cd /` — reset to the VFS root (see [The Root Directory](#the-root-directory)).
- `cd /mnt` — the directory holding the mount points.
- `cd ..` — go to parent; at the root it stays at the root.
- `cd <name>` — relative to the current directory: a mount point, or a directory on FAT12 or ISO9660.
- `cd /mnt/fat/<path>` — absolute FAT12 path (only with a FAT12 floppy).
- `cd /mnt/tmp/<path>` — absolute path on the FAT16 RAM disk.
- `cd /mnt/iso/<path>` — absolute ISO9660 path (validates directory exists).

Multi-component paths (`foo/bar`, `../bar`) are supported. A path that is neither `/`, nor on the way to a mount, nor on a mounted filesystem is refused with `no such directory`.

The working directory is kept in a 32-byte field that userland also reads back
through `sysinfo`, so a path longer than that is refused with `cd: path too
long` instead of being stored truncated.

### `cls`

Clears the screen (fills framebuffer/VGA buffer with black).

### `debug` *(hidden)*

Writes the in-memory debug log (`debug!`/`debugln!`, 8 KiB) to `DEBUG.TXT` in the floppy's root when there is a FAT12 floppy, to `/mnt/tmp/KERNDBG.LOG` on the RAM disk, and to the serial port (COM1). Each file is replaced, so `KERNDBG.LOG` written at boot (see [Init](init/overview.md#debug-log)) is brought up to date.

```
debug
read /mnt/tmp/KERNDBG.LOG
```

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

At `/` and `/mnt` it lists the mount table instead: a directory on the way to mounts as `[ DIR ]`, a mount point as `[MOUNT]` with its filesystem type, as `mount` names it. On a machine without a floppy, `fat` is not there.

```
root@rourex:/ > dir
 mnt            [ DIR ]
root@rourex:/ > cd mnt
root@rourex:/mnt > dir
 fat            [MOUNT] fat12
 tmp            [MOUNT] memdisk
 iso            [MOUNT] iso9660
 tar            [MOUNT] tar
```

### `echo <text>`

Prints the argument string followed by a newline.

```
echo hello world
```

### `fsck`

Runs the FAT12 filesystem check (`fs::fat12::check::run_check`). Prints a report with error count, orphaned clusters, cross-linked clusters, and invalid entries.

### `hda`

What the HD Audio driver found: the controller's PCI address and ids, whether commands go through CORB/RIRB or the immediate registers, the codec, its audio function group, and the DACs and output pins in use (line out, speaker, headphones); while a stream plays, its rate and how much has been played, is queued, and how often it ran dry. `hda tone` plays a second of 440 Hz, which is the quickest way to hear whether sound works on a machine. See [HD Audio](audio/hda.md).

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

Powers the machine off (ACPI S5). The sleep type comes from the `\_S5_` package in the DSDT and goes to the PM1a/PM1b control ports the FADT names; if the firmware has not switched to ACPI mode yet, the kernel asks it to first (`SMI_CMD` ← `ACPI_ENABLE`). Without usable tables it falls back to the fixed QEMU/Bochs ports, then halts. See `src/acpi/`.

### `reboot`

Restarts the machine: the FADT's reset register when the firmware has one, then the chipset's reset control port (`0xCF9`), then the keyboard controller (`0x64` ← `0xFE`), and last a triple fault.

### `nic`

Shows the selected NIC, register base, MAC and registered driver slot, plus delivered, held, dropped and missed-frame counters. Intel cards also report link speed/duplex, PHY state, hardware counters and descriptor-ring positions. Start `eth` first to register the driver.

Intel diagnostics accept decimal numbers or hexadecimal with `0x`:

```sh
nic reset             # reinitialise the card
nic reset phy         # also reset the PHY; link recovery takes time
nic tx                # queue one test frame and show descriptor progress
nic reg <offset> [value]
nic phy <page> <register> [value]
nic nvm <word>        # read a PCH flash NVM word
```

`reg` and `phy` read when no value is supplied, or write and read back when one is supplied. These operations, including reset and test transmission, require the Intel backend; RTL8139 still has the basic card and routing-counter display.

### `acpi`

Shows what `hlt` and `reboot` will use on this machine: the root table GRUB passed, the PM1 control ports, the ACPI mode switch, the `\_S5_` sleep types and the reset register. Useful to photograph when power-off or restart does not work on a particular board.

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

Creates a subdirectory in the current FAT12 directory. Name is uppercased to 8.3 format. Maximum name length: 11 bytes. Refused with `not on a writable volume` when the working directory is on no FAT12 volume (`/`, `/mnt`, the ISO or the archive).

```
mkdir MYDIR
```

### `mount`

Lists all active VFS mount table entries, one line per mount: `<path> (<fstype>[, <format>][, <size>][, <free> free])`. The format is there when it says more than the type (the RAM disk is a `memdisk` and `fat16`); the size is the whole volume, and the free space is shown on the writable ones. Sizes are in whole MiB from 10 MiB, in KiB from 10 KiB. See [Mount Sizes](filesystem/overview.md#mount-sizes-fsusage-syscall-0x40). `/mnt/fat` is there only when a FAT12 floppy was found at boot.

```
/ (rootfs)
/mnt/fat (fat12, 1440 KiB, 1338 KiB free)
/mnt/tmp (memdisk, fat16, 126 MiB, 125 MiB free)
/mnt/iso (iso9660, 112 MiB)
/mnt/tar (tar, 48 MiB)
```

### `mv <old> <new>`

Renames a file in the current FAT12 directory. Both names are converted to 8.3 format. Does not change the file's data or cluster chain. Refused with `not on a writable volume` outside a FAT12 volume, as `mkdir` is.

```
mv FOO.TXT BAR.TXT
```

### `read <filename>`

Prints the contents of a file. Supports both FAT12 (relative or absolute) and ISO9660 paths. Reads up to 4096 bytes; a longer file is cut short. A name that lies on no mounted filesystem (a bare name at `/`, say) answers `no such file`.

```
read HELLO.TXT
read /mnt/fat/GARN/INDEX.HTM
read /mnt/tmp/SUB/NOTE.TXT
read /mnt/iso/readme.txt
```

### `reset`

Force resets the VGA video mode to `0x03` (text mode).

### `rm <filename>`

Deletes a file from the current FAT12 directory: its cluster chain is returned to the FAT, then the directory entry is marked `0xE5` (deleted). Directories are not matched. Outside a FAT12 volume it answers `no such file`; it used to delete from the floppy's root there, even with the working directory on the ISO.

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

### `write <name> <text>`

Writes `<text>` to `<NAME>.TXT` in the current FAT12 directory, replacing the file if it exists. The name is at most 8 characters, and `.TXT` is always appended. It works on the floppy and on the RAM disk at `/mnt/tmp`, and is refused with `not on a writable volume` anywhere else.

```
cd /mnt/tmp
write notes hello from ram
read NOTES.TXT
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
2. Finds the ELF file — `/mnt/tmp/jug` first for [Jug](sdk/jug.md) downloads, then the current working directory (on FAT, ISO9660 or the archive), then `/mnt/tar/bin` and `/mnt/iso/bin` — and stages it in a temporary user-heap buffer.
3. Copies the `PT_LOAD` segments into the slot's private 2 MiB physical frame and builds a page table that maps it at `0x600_000`.
4. Pushes the argv frame onto the slot's initial user stack and creates the scheduler task at the ELF entry point.
5. `Foreground`: the launcher (the shell, or `init_rc`) is recorded as the child's waiter and parked until the child ends.
6. `Background`: returns immediately; the shell stays interactive.

Userland processes communicate with the kernel via interrupt `0x7F` (syscall gate). See [Syscall specification](abi/syscall_specification.md) for the full syscall interface, and the [SDK](sdk/index.md) for building programs.
