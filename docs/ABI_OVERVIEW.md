# Application Binary Interface (ABI)

This overview document presents the rou2exOS (aka `r2`) kernel interface for external applications. Applications should utilize custom programming language libraries provided in [the apps repository](https://github.com/krustowski/rou2exOS-apps) and statically link them with their source code. Examples on how to use such libraries, how to compile them and link them are provided in directories named by concerned languages.


## Syscall Specification

The system call (syscall) is a procedure for requesting or modifying of kernel components, modules and drivers. Syscalls use the software interrupts (`int 0x7f`) under the hood to notify the CPU and kernel to take an action. Parameters of a syscall are passed using the CPU registers that are listed below.

Please note that all values passed into a syscall must be aligned to 8 bytes (64bit).

| Register | Usage          | Example value (64bit) |
|----------|----------------|-----------------------|
| `RAX`    | syscall No.    | `0x01` |
| `RDI`    | argument No. 1 | `0x01` |
| `RSI`    | argument No. 2 | `0x100aaa` |


### Table of Syscalls

Please note that these lists are incomplete as listed syscalls have to be implemented in the kernel ABI. To be expanded.

#### System Information and Memory Management

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x00`|  process ID | program return code | Graceful exit of a program/process/task. | ✅ |
|  `0x01`|  `0x01`|  pointer to SysInfo struct | Get the system information summary. Pointer in arg. 2 has to be casted to the SysInfo struct provided by a language library. Memory must be already allocated. | ✅ |
|        |  `0x02`|  pointer to SysInfo struct | Set the system information summary. Pointer in arg, 2 is a pointer to the SysInfo structure with new information items. | ❌ |
|  `0x02`|  `0x01`| pointer to RTC struct | Get the Real Time Clock (RTC) data. | ❌ | 
|  `0x0f`|  pointer to type pointer | size in bytes to allocate | Allocate a memory block on heap. The pointer to the allocated block is returned in `RAX`, or is `0x00` if the allocation procedure fails. | ❌ |

#### Video Output

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x10`|  pointer to string data | string length | Print provided string to terminal. | ✅ |
|  `0x11`|  `0x00` | `0x00` | Clear the screen. | ✅ |

#### Filesystem (FAT12)

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x20`|  pointer to file name string | pointer to buffer | Read a file specified in the first argument and load its contents into the buffer in argument 2. | ✅ |
|  `0x21`|  pointer to string data | pointer to buffer | Write the buffer into a file (overwrite it) specified by the first argument. File is created in the current directory if not exists. | ✅ |
|  `0x22`|  pointer to string data | pointer to string data | Rename the file specified by its name in argument No. 1 to value specified in argument No. 2. | ❌ |
|  `0x23`|  pointer to string data | --- | Delete the file specified in argument No. 1. Applicable on a file in the current directory. | ❌ |
|  `0x24`|  cluster No. | pointer to next cluster No. int64 |  Read the FAT table and find next (or first) sector of provided cluster. | ❌ |
|  `0x25`|  cluster No. | value | Write into given cluster such value provided in the argument No. 2. | ❌ |
|  `0x26`|  cluster NO. | pointer to the Entry structure | Insert an Entry provided via the first argument into the directory with Cluster No. specified in the argument No. 2. | ❌ |
|  `0x27`|  cluster No. (current directory usually) | pointer to string data | Create a new subdirectory in such parent directory specified by name in argument No. 2. | ✅ |
|  `0x28`|  cluster No. | pointer to array of entries | List the current directory. | ✅ |
|  `0x29`|  pointer to file name string | pointer to uint64 (PID) | Execute a flat binary executable (.BIN usually). | ❌ |
|  `0x2a`|  pointer to file name string | pointer to uint64 (PID) | Execute an ELF64 executable (.ELF). | ❌ |

#### Port I/O and Networking

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x30`|  port identificator (ID) | pointer to value (uint64) | Send a value to a port specified in arg No. 1. | ✅ |
|  `0x31`|  port identificator (ID) | pointer to value (uint64) | Receive a value from a port specified in arg No. 1. | ✅ |
|  `0x32`|  `0x01`|  pointer to array of network devices | List all network devices/interfaces available. | ❌ |
|  `0x33`|  `0x01`|  pointer to buffer | Create a new ICMP packet.  | ❌ |

#### Audio

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x40`|  frequency in Hz | length in milliseconds | Play the frequency. | ❌ |
|  `0x41`|  `0x01`| pointer to the audio file | Play the audio file. | ❌ |
|  `0x4f`|  `0x00`|  `0x00`| Stop the player. | ❌ |


### Syscall Return Codes

| Code (uint64) | Meaning |
|---------------|---------|
| `0x00` | `Okay` |
| `0xfb` | `NotImplemented` |
| `0xfc` | `InvalidInput` |
| `0xfd` | `FilesystemError` |
| `0xfe` | `FileNotFound` |
| `0xff` | `InvalidSyscall` |


### Type Definitions

#### SysInfo

```rust 
pub struct SysInfo {
    pub system_name: [u8; 32],
    pub system_user: [u8; 32],
    pub system_path: [u8; 32],
    pub system_version: [u8; 8],
    pub system_uptime: u32,
}
```

```c
typedef struct {
    uint8_t system_name[32];
    uint8_t system_user[32];
    uint8_t system_path[32];
    uint8_t system_version[8];
    uint32_t  system_uptime;
} __attribute__((packed)) SysInfo_T;
```

#### RTC 

```rust
#[repr(C, packed)]
pub struct RTC {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day: u8,
    pub month: u8,
    pub year: u8,
    pub century: u8,
}
```

#### Entry (FAT12)

```rust
#[repr(C, packed)]
#[derive(Default,Copy,Clone)]
pub struct Entry {
    pub name: [u8; 8],
    pub ext: [u8; 3],
    pub attr: u8,
    pub reserved: u8,
    pub create_time_tenths: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub last_access_date: u16,
    pub high_cluster: u16,
    pub write_time: u16,
    pub write_date: u16,
    pub start_cluster: u16,
    pub file_size: u32,
}
```

```c
#pragma pack(push, 1)
typedef struct {
    uint8_t name[8];
    uint8_t ext[3];
    uint8_t attr;
    uint8_t reserved;
    uint8_t tenths;
    uint16_t create_time;
    uint16_t create_date;
    uint16_t last_access_time;
    uint16_t high_cluster;
    uint16_t write_time;
    uint16_t write_date;
    uint16_t start_cluster;
    uint32_t file_size;
} Entry_T;
#pragma pack(pop)
```

