# Application Binary Interface (ABI)

This overview document presents the rou2exOS (aka `r2`) kernel interface for external applications. Applications should utilize custom programming language libraries provided in [the apps repository](https://github.com/krustowski/rou2exOS-apps) and statically link them with their source code. Examples on how to use such libraries, how to compile them and link them are provided in directories named by concerned languages.

## Syscall Specification

The system call (syscall) is a procedure for requesting or modifying of kernel components, modules and drivers. Syscalls use the software interrupts (`int 0x7f`) under the hood to notify the CPU and kernel to take an action. Parameters of a syscall are passed using the CPU registers that are listed below.

Please note that all values passed into a syscall must be aligned to 8 bytes (64bit).

| Register | Usage          | Example value (64bit) |
|----------|----------------|-----------------------|
| `RAX`    | syscall No.    | `0x01` |
| `RDI`    | argument No. 1 | `0x01` |
| `RSI`    | argument No. 2 | `0x1000a` |

### Table of Syscalls

Please note that this list is incomplete as listed syscalls have to be implemented in the kernel ABI. To be expanded.

| Syscall No. | Argument 1 | Argument 2 | Purpose |
|-------------|------------|------------|---------|
|  `0x01`|  `0x01`|  pointer to SysInfo struct | Get the system information summary. Pointer in arg. 2 has to be casted to the SysInfo struct provided by a language library. Memory must be already allocated. |
|  `0x0f`|  pointer to type pointer | size in bytes to allocate | Allocate a memory block on heap. The pointer to the allocated block is returned in `RAX`, or is `0x00` if the allocation procedure fails. |
|  `0x10`|  pointer to string data | string length | Print provided string to terminal. |
|  `0s20`|  pointer to file name string | pointer to buffer | Read a file specified in the first argument and load its contents into the buffer in argument 2. |

[...]

### Type Definitions

```rust 
pub struct SysInfo {
    pub system_name: [u8; 32],
    pub system_user: [u8; 32],
    pub system_version: [u8; 8],
    pub system_uptime: u32,
}

[...]
