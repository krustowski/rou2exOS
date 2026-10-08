# System, Processes and Memory Management

## 0x42 (Coordinated desktop relaunch)

Coordinates replacement of the graphics session's foreground `MEMENTO.ELF`.
The kernel's `init_rc` desktop launcher registers as its supervisor. A Memento
started through another launcher is not eligible.

| Argument 1 | Argument 2 | Meaning |
|------------|------------|---------|
| `0` | Memento PID from `0x2f` | Request a relaunch; requires registered support. |
| `1` | `0` | Poll from Memento: returns `1` for a pending request, `0` otherwise. |
| `2` | `0` | Memento registers that it supports this protocol. |

Registration and requests return `0` on success. Errors are `0xfa` (scheduler
busy), `0xfb` (unsupported desktop/supervisor), `0xfc` (invalid operation or
command line), and `0xfe` (unknown PID). An older kernel returns `0xff`.
libc++r2 provides `register_desktop_relaunch()`,
`desktop_relaunch_pending()` and `request_desktop_relaunch(pid)`.

A request does not kill or spawn a process. Memento polls outside window
callbacks, closes its desktop and destroys the UI root, stopping hosted children
before their shared memory is released. It then exits through `0x00` with
return code `0x4d52` (`r2::DesktopRelaunchExit`). Only this graceful exit with a
pending request commits a one-shot command to the supervising launcher. An
ordinary exit, kill or crash retains the normal graphics reboot policy.

The launcher starts Memento again through executable lookup, preferring the
Jug download, with its original arguments and one `--relaunch` flag. This
skips the welcome screen and returns to login. Neither `INIT.RC` nor the boot
initialization runs again, so the RAM disk and `SESSION.CFG` remain available.

## 0x41 (Process command line)

Returns the saved command line of a process, including `argv[0]`. Jug uses it
to restart every instance with its original arguments. Kernel tasks have an
empty command line.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| PID from `0x2f` | pointer to a 128-byte output buffer | ✅ |

The return value is the byte length (`0..128`); the output is not
NUL-terminated. Returns `FileNotFound` (`0xfe`) for an unknown PID, `Busy`
(`0xfa`) when the scheduler is locked, or `InvalidInput` for an invalid buffer.

## 0x00 (Graceful Program Exit)

The process'/task's ID is resolved by the kernel scheduler automatically. The process is killed, its page tables and any heap blocks it still owns are released, and a launcher waiting on it (`fg`) is woken.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| *unused*  | program return code | ✅ |

## 0x01 (System Information)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01`   | pointer to `SysInfo` struct | Read the system information summary. | ✅ |
| `0x02`   | pointer to `SysInfo` struct | Write the system information summary. (Currently, only `ip_addr` fields is written back from the struct; all other fields are ignored.) | ✅ |
| `0x03`   | pointer to `SysInfo` struct | Set the system user from `system_user`, up to its NUL or all 32 bytes; every other field is ignored. The name must be one word of printable ASCII (`0x21`–`0x7e`), or the call returns `InvalidInput`. Memento sets it from its login. | ✅ |

`system_path` and `system_user` are NUL-terminated after their text. The call returns `Busy` (`0xfa`) instead of an untouched buffer when the system configuration lock cannot be taken.

## 0x02 (Real-Time Clock)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01`   | pointer to `RTC` struct | Read the system time and date. | ✅ |
| `0x02`   | pointer to `RTC` struct | Write the system time and date. | ❌ |

## 0x03 (Pipe handling)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01`   | pointer to circular buffer | Register a buffer to receive scancodes from IRQ1. | ✅ |
| `0x02`   | pointer to circular buffer | Unregister a buffer from receiving any scancodes from IRQ1. | ✅ |
| `0x03`   | pointer to circular buffer | Read from the registered buffer (IRQ1). | ✅ |
| `0x04` | *unused* | Register current process to receive mouse packets (IRQ12). | ✅ |
| `0x05` | pointer to output buffer | Drain up to 5 complete 3-byte packets (15 bytes) from the mouse ring buffer into the caller's buffer. Returns bytes written (always a multiple of 3). | ✅ |
| `0x06` | *unused* | Unregister current process from receiving mouse packets (IRQ12). | ✅ |

## 0x04 (Tick count in milliseconds)

Get millisecond tick count since boot. Returns elapsed milliseconds in `RAX`. The nominal resolution is 1 ms at the current 1000 Hz PIT rate (`TICKS_PER_SECOND`). The current implementation also advances the counter for software yields through `int 0x20`, so it can run ahead of wall time.

No argument is used. The syscall is implemented.

## 0x05 (Sleep)

Sleep for at least the given number of milliseconds. Rounded up to the next PIT tick (1 ms at 1000 Hz). Marks the calling process as Blocked; the scheduler wakes it automatically — no busy-wait.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| duration in milliseconds   | *unused* | ✅ | 

## 0x0a (Allocate memory on heap)

Allocate a block from the userland heap (`0xc00000`-`0xffffff`). Returns the virtual address of the zeroed block in `RAX` as response, or `0x00` on failure.

When the 4 MiB are full, the heap grows once: an extension of an eighth of the RAM (at least 2 MiB, below 1 GiB) is added at `0xa000000`, or past the tar archive when that lies there, mapped user-accessible in every process, and the allocation is served from it. A block can therefore come from either region; both are memory like any other to the caller. See [Allocators](../../memory/allocators.md#growing-the-extension) for the details, and `0x3c` for where the heap stands.

The heap is shared by all processes. Each block is tagged with the slot of the process that allocated it and is freed automatically when that process exits, is killed or crashes.

A heap block can be passed to any syscall that takes a pointer, as long as the buffer the call uses fits inside the block's region; see [Pointer Arguments](../syscall_specification.md#pointer-arguments).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| size in bytes | *unused* | ✅ | 

## 0x0b (Reallocate memory on heap)

Reallocate a heap block. Tries in-place expansion first; falls back to allocate+copy+free. Returns the (possibly new) address, or `0x00` on failure. The block keeps its original owner.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to existing block (or `0x00`) | new size in bytes | ✅ |

## 0x0f (Free a heap block)

Free a heap block. Immediately coalesces adjacent free blocks. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to block | `0x00` | ✅ |

## 0x3b (Kill process)

Kill a process by the PID reported by `0x2f` (and printed by the shell's `ts`), not by scheduler slot. The target's page tables and heap blocks are released and its launcher, if parked on it, is woken.

Returns `0x00` on success, or `FileNotFound` (`0xfe`) when no live process has that PID.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| PID | *unused* | ✅ |

## 0x3c (Memory information)

Fills a [`MemInfo`](../type_definitions.md#meminfo-syscall-0x3c) with the usable RAM, the layout of the per-process frames, and the user heap (`0x0a`) added up block by block --- including how many bytes each process slot holds, and which task id sits in each slot, so the figures can be put next to `0x2f`'s list. The same numbers the kernel shell's `heap` and `meminfo` print.

Argument 2 picks the layout. `0` or `1` fills a `MemInfo`, which has room for 16 slots: what every program built before there were 32 slots passes. The heap held by slots 16 and up is counted in its untagged entry, so the figures still add up. `2` fills a [`MemInfo2`](../type_definitions.md#meminfo2-syscall-0x3c-version-2) with room for all 32.

Returns `0x00`, `InvalidInput` (`0xfc`) for a pointer outside the user regions or an unknown version, or `Busy` (`0xfa`) when the heap or the scheduler is locked at that moment; ask again.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to `MemInfo` or `MemInfo2` | version: `0`/`1` or `2` | ✅ |

## 0x3e (Power)

Restarts or switches off the machine. Neither comes back: `acpi::shutdown` tries the reset register or sleep state the ACPI tables name first, and then the chipset's reset control (`0xcf9`), the keyboard controller and a triple fault. Anything else in Argument 1 returns `InvalidInput` (`0xfc`); a kernel older than the call returns `InvalidSyscall` (`0xff`), which is how a caller tells that it is still running because the call is not there.

Memento uses it when its login dialog is left: that is the end of the session on the graphics kernel, where there is no shell to go back to.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x01` restart, `0x02` power off | *unused* | ✅ |
