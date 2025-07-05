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

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command |
|-------------|------------|------------|---------|
|  `0x01`|  `0x01`|  pointer to SysInfo struct | Get the system information summary. Pointer in arg. 2 has to be casted to the SysInfo struct provided by a language library. Memory must be already allocated. |
|  `0x0f`|  pointer to type pointer | size in bytes to allocate | Allocate a memory block on heap. The pointer to the allocated block is returned in `RAX`, or is `0x00` if the allocation procedure fails. |
|  `0x10`|  pointer to string data | string length | Print provided string to terminal. |
| --- | --- | --- | **Filesystem operations** |
|  `0x20`|  pointer to file name string | pointer to buffer | Read a file specified in the first argument and load its contents into the buffer in argument 2. |
|  `0x21`|  pointer to string data | pointer to buffer | Write the buffer into a file (overwrite it) specified by the first argument. File is created in the current directory if not exists. |
|  `0x22`|  pointer to string data | pointer to string data | Rename the file specified by its name in argument No. 1 to value specified in argument No. 2. |
|  `0x23`|  pointer to string data | --- | Delete the file specified in argument No. 1. Applicable on a file in the current directory. |
|  `0x24`|  cluster No. | pointer to next cluster No. int64 |  Read the FAT table and find next (or first) sector of provided cluster. |
|  `0x25`|  cluster No. | value | Write into given cluster such value provided in the argument No. 2. |
|  `0x26`|  pointer to the Entry structure | cluster No. (directory cluster) | Insert an Entry provided via the first argument into the directory with Cluster No. specified in the argument No. 2. |
|  `0x27`|  parent cluster No. (current directory usually) | pointer to string data | Create a enw subdirectory in such parent directory specified by name in argument No. 2. |
|  `0x28`|  parent cluster No. | pointer to array of entries | List the current directory. |

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
