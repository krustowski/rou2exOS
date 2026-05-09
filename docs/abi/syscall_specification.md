# Overview

This overview document presents the rou2exOS (aka `r2`) kernel interface for external applications. Applications should utilize custom programming language libraries provided in [the apps repository](https://github.com/krustowski/r2apps) and statically link them with their source code. Examples on how to use such libraries, how to compile them and link them are provided in directories named by concerned languages.

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

Please note that all values passed into a syscall must be aligned to 8 bytes (64bit).

| Register | Usage          | Example value (64bit) |
|----------|----------------|-----------------------|
| `RAX`    | syscall No.    | `0x01` |
| `RDI`    | argument No. 1 | `0x01` |
| `RSI`    | argument No. 2 | `0x123abc` |

### Syscall Return Codes

| Code (uint64) | Meaning |
|---------------|---------|
| `0x00` | `Okay` |
| `0xfb` | `NotImplemented` |
| `0xfc` | `InvalidInput` |
| `0xfd` | `FilesystemError` |
| `0xfe` | `FileNotFound` |
| `0xff` | `InvalidSyscall` |
