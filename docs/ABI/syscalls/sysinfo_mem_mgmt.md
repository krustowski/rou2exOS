# System Information and Memory Management

## 0x00 (Graceful Program Exit)

The process'/task's ID is resolved by the kernel scheduler automatically.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| *unused*   | program return code | ✅ |

## 0x01 (System Information)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01`   | pointer to `SysInfo` struct | Read the system information summary. | ✅ |
| `0x02`   | pointer to `SysInfo` struct | Write the system information summary. (Currently, only `ip_addr` fields is written back from the struct; all other fields are ignored.) | ✅ |

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
| `0x04` | pointer to circular buffer | Drains up to 5 complete 3-byte packets (15 bytes) from the mouse ring buffer into the caller's buffer. Returns: bytes written (always a multiple of 3). | ✅ |
| `0x06` | *unused* | Unregister current process from receiving mouse packets (IRQ12). | ✅ |

## 0x04 (Tick count in milliseconds)

Get millisecond tick count since boot. Returns elapsed milliseconds in `RAX` (10 ms resolution at 100 Hz PIT).

No argument is used. The syscall is implemented.

## 0x05 (Sleep)

Sleep for at least the given number of milliseconds. Rounded up to the next 10 ms PIT tick. Marks the calling process as Blocked; the scheduler wakes it automatically — no busy-wait.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| duration in milliseconds   | *unused* | ✅ | 

## 0x0a (Allocate memory on heap)

Allocate a block from the userland heap (`0xc00000`-`0xffffff`). Returns the virtual address of the zeroed block in `RAX` as response, or `0x00` on failure.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| size in bytes | *unused* | ✅ | 

## 0x0b (Reallocate memory on heap)

Reallocate a heap block. Tries in-place expansion first; falls back to allocate+copy+free. Returns the (possibly new) address, or `0x00` on failure. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to existing block (or `0x00`) | new size in bytes | ✅ |

## 0x0f (Free a heap block)

Free a heap block. Immediately coalesces adjacent free blocks. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to block | `0x00` | ✅ | 

