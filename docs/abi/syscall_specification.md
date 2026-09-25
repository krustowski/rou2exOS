# Overview

This overview document presents the rou2exOS (aka `r2`) kernel interface for external applications. Applications should use one of the language libraries from [the apps repository](https://github.com/krustowski/rou2exOS-apps) and link it statically: [libcr2](../sdk/libcr2.md) for C, [libc++r2](../sdk/libcxxr2.md) for C++, or [libgor2](../sdk/libgor2.md) for Go. See the [SDK Overview](../sdk/index.md) for the rules every program has to follow.

## Privilege Levels

The privilege levels are to be specified and defined in the Global Descriptor Table (GDT) in early boot sequence procedures.

| CPU Ring | Common Interrupt | Target purpose |
|----------|------------------|----------------|
| `0` | *      | kernel space |
| ~~`1`~~ | ~~`0x7d`~~ | kernel tasks, drivers, kernel services |
| ~~`2`~~ | ~~`0x7e`~~ | privileged user space, services, privileged shell access |
| `3` | `0x7f` | user space, user programs, common shell |

**Kernel itself handles multiple software (CPU exceptions) and hardware interrupts (e.g. IRQs).*

All common interrupts are callable from anywhere, but are handled only when called from the defined CPU ring, therefore are locked to such space.

## Syscall Specification

The system call (syscall) is a procedure for requesting or modifying of kernel components, modules and drivers. Syscalls use the software interrupts (`int 0x7f`) under the hood to notify the CPU and kernel to take an action. Parameters of a syscall are passed using the CPU registers that are listed below.

All values passed into a syscall are 64-bit.

| Register | Usage          | Example value (64bit) |
|----------|----------------|-----------------------|
| `RAX`    | syscall No.    | `0x01` |
| `RDI`    | argument No. 1 | `0x01` |
| `RSI`    | argument No. 2 | `0x123abc` |
| `RAX`    | return value   | `0x00` |

### Entry Stub

The interrupt gate for `0x7f` is a naked stub (`syscall_handler` in `src/abi/syscall.rs`) that saves the general-purpose registers, calls the dispatcher `syscall_inner(arg1, arg2, syscall_no)` and returns with `iretq`:

```
mov rcx, rdx         ; (legacy)
push rax ... r15     ; every GPR except R9
mov rdx, rax         ; syscall number becomes the 3rd C argument
call syscall_inner
mov r9, rax          ; keep the result across the pops
pop r15 ... rax
mov rax, r9          ; result into RAX
iretq
```

Consequences for callers:

- **The number is read from `RAX`.** Setting `RDX` alone is not enough; libc++r2 sets both to be safe.
- **There are only two arguments.** Anything in `RDX` or `RCX` is not seen by the dispatcher. Syscalls that need more take a pointer to a request struct (e.g. `0x39`, `0x3a`).
- **`R9` is clobbered** by every syscall and must be declared as such in inline assembly.
- **Interrupts are re-enabled** at the top of the dispatcher, so a long syscall can be preempted by the scheduler.

### Pointer Arguments

Every syscall that takes a pointer checks the whole buffer it will touch, not just its first byte. The buffer must lie wholly inside one user region, either the program image and stack (`0x600_000..0xA00_000`) or the userland heap (`0xC00_000..0x1000_000`, handed out by syscall `0x0a`). Otherwise the call fails with `InvalidInput` (or the call's own error value). A buffer may not straddle the gap between the two regions. NUL-terminated strings are read up to the end of the region they start in.

### Syscall Return Codes

Most syscalls return one of these codes. Syscalls that return a count, an address or a PID document their own return value; several of them use `u64::MAX` (`-1` as `int64_t`) for an error.

| Code (uint64) | Meaning |
|---------------|---------|
| `0x00` | `Okay` |
| `0xfa` | `Busy` — a kernel lock could not be taken in time; retry |
| `0xfb` | `NotImplemented` |
| `0xfc` | `InvalidInput` |
| `0xfd` | `FilesystemError` |
| `0xfe` | `FileNotFound` (also used for "no such process") |
| `0xff` | `InvalidSyscall` |

### Syscall Map

| Range | Group | Page |
|-------|-------|------|
| `0x00`–`0x0f` | Exit, system information, pipes, time, heap, kill (`0x3b`), memory information (`0x3c`) | [System, Processes & Memory](syscalls/sysinfo_mem_mgmt.md) |
| `0x10`–`0x1f` | Console, graphics, audio | [Video & Audio](syscalls/video_audio.md) |
| `0x20`–`0x2f`, `0x39`–`0x3a` | Files, directories, VFS, program execution, task list | [Filesystem](syscalls/filesystem.md) |
| `0x30`–`0x38` | I/O ports, serial, packets, IPC, networking | [Ports & Networking](syscalls/port_networking.md) |
