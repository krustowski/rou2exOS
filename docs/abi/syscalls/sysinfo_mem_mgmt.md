# System, Processes and Memory Management

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

`system_path` is NUL-terminated after the working directory. The call returns `Busy` (`0xfa`) instead of an untouched buffer when the system configuration lock cannot be taken.

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

Get millisecond tick count since boot. Returns elapsed milliseconds in `RAX`. The resolution is one PIT tick, which is 1 ms at the current 1000 Hz rate (`TICKS_PER_SECOND`).

No argument is used. The syscall is implemented.

## 0x05 (Sleep)

Sleep for at least the given number of milliseconds. Rounded up to the next PIT tick (1 ms at 1000 Hz). Marks the calling process as Blocked; the scheduler wakes it automatically — no busy-wait.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| duration in milliseconds   | *unused* | ✅ | 

## 0x0a (Allocate memory on heap)

Allocate a block from the userland heap (`0xc00000`-`0xffffff`). Returns the virtual address of the zeroed block in `RAX` as response, or `0x00` on failure.

The heap is shared by all processes. Each block is tagged with the slot of the process that allocated it and is freed automatically when that process exits, is killed or crashes.

Heap addresses lie outside the range syscalls accept for pointer arguments, so a heap block cannot be passed to e.g. `0x10` or `0x20` directly; see [Pointer Arguments](../syscall_specification.md#pointer-arguments).

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
