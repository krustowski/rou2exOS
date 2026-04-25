# Application Binary Interface (ABI)

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

### Table of Syscalls (int 0x7f)

Please note that these lists are incomplete as listed syscalls have to be implemented in the kernel ABI. To be expanded.

#### System Information and Memory Management

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x00`|  process ID | program return code | Graceful exit of a program/process/task. | ✅ |
|  `0x01`|  `0x01`|  pointer to SysInfo struct | Get the system information summary. Pointer in arg. 2 has to be casted to the SysInfo struct provided by a language library. Memory must be already allocated. | ✅ |
|        |  `0x02`|  pointer to SysInfo struct | Set the system information summary. Pointer in arg, 2 is a pointer to the SysInfo structure with new information items. | ❌ |
|  `0x02`|  `0x01`| pointer to RTC struct | Get the Real Time Clock (RTC) data. | ✅ |
|  `0x03`|  `0x01`| pointer to circular buffer | Register a buffer to receive scancodes from IRQ1 | ✅ |
|        |  `0x02`| pointer to circular buffer | Unregister a buffer to receive scancodes from IRQ1 | ✅ |
|        |  `0x03`| pointer to circular buffer | Read from the buffer. | ✅ |
|  `0x0a`|  pointer to type pointer | size in bytes to allocate | Allocate a memory block on heap. The pointer to the allocated block is returned in `RAX`, or is `0x00` if the allocation procedure fails. | ❌ |
|  `0x0b`|  pointer to type pointer | size in bytes to allocate | Reallocate the memory block on heap. | ❌ |
|  `0x0f`|  pointer to type pointer | `0x00` | Free the allocated memory on heap. | ❌ |

#### Video + Audio Output

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x10`|  pointer to string data | string length | Print provided string to terminal. | ✅ |
|  `0x11`|  `0x00` | `0x00` | Clear the screen. | ✅ |
|  `0x12`|  encoded position | encoded color | Write a graphical pixel. | ✅ |
|  `0x13`|  a 320×200 VGA mode-13h palette-indexed buffer | pointer to RGB or default VGA palette | Write a VGA buffer into kernel framebuffer. | ✅ |
|  `0x14`|  reserved (`0x00`) | reserved (`0x00`) | Maps physical VGA graphics RAM (0xA0000–0xAFFFF) into the calling process at virtual 0xA00_000 with USER+WRITE. Returns the virtual base address on success, 0 on failure.. | ✅ |
|  `0x15`|  video mode | reserved (`0x00`) | Programs VGA hardware registers for the given mode. | ✅ |
|  `0x1a`|  frequency in Hz | length in milliseconds | Play the frequency. | ✅ |
|  `0x1b`|  `0x01`| pointer to the audio file | Play the MIDI audio file. | ✅ |
|  `0x1f`|  `0x00`|  `0x00`| Stop the player. | ✅ |

#### Filesystem (VFS / FAT12)

File name arguments accept either a bare name relative to the current working directory (e.g. `FOO.TXT`) or an absolute VFS path (e.g. `/mnt/fat/FOO.TXT`).  Both forms are resolved through the VFS mount table.

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x20`|  pointer to file name string | pointer to buffer | Read a file specified in the first argument and load its contents into the buffer in argument 2. | ✅ |
|  `0x21`|  pointer to string data | pointer to buffer | Write the buffer into a file (overwrite it) specified by the first argument. File is created in the current directory if not exists. | ✅ |
|  `0x22`|  pointer to string data | pointer to string data | Rename the file specified by its name in argument No. 1 to value specified in argument No. 2. | ✅ |
|  `0x23`|  pointer to string data | `0x00` | Delete the file specified in argument No. 1. Applicable on a file in the current directory. | ✅ |
|  `0x24`|  cluster No. | pointer to next cluster No. int64 |  Read the FAT table and find next (or first) sector of provided cluster. | ❌ |
|  `0x25`|  cluster No. | value | Write into given cluster such value provided in the argument No. 2. | ❌ |
|  `0x26`|  cluster No. | pointer to the Entry structure | Insert an Entry provided via the first argument into the directory with Cluster No. specified in the argument No. 2. | ❌ |
|  `0x27`|  cluster No. (current directory usually) | pointer to string data | Create a new subdirectory in such parent directory specified by name in argument No. 2. | ✅ |
|  `0x28`|  cluster No. | pointer to array of entries | List the current directory. | ✅ |
|  `0x29`|  pointer to file name string | pointer to uint64 (PID) | Execute a flat binary executable (.BIN usually). | ❌ |
|  `0x2a`|  pointer to file name string | pointer to uint64 (PID) | Execute an ELF64 executable (.ELF). Auto-appends `.elf` if no extension given. | ✅ |
|  `0x2b` | unused | pointer to `FsckReport_T` | Run the FAT12 filesystem check; populates the report struct pointed to by arg2. | ✅ |
|  `0x2c` | unused | pointer to array of up to 8 `MountInfo_T` | List VFS mount points. Returns the number of active mounts as a u64. `fs_type`: `0`=none, `1`=rootfs, `2`=fat12. | ✅ |

#### Port I/O and Networking

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x30`|  port identificator (ID) | pointer to value (uint64) | Send a value to a port specified in arg No. 1. | ✅ |
|  `0x31`|  port identificator (ID) | pointer to value (uint64) | Receive a value from a port specified in arg No. 1. | ✅ |
|  `0x32`|  `0x01` |  `0x00` | Serial port (UART, COM1) initialization. | ✅ |
|        |  `0x02` |  pointer to value | Read from the serial port (UART, COM1). | ✅ |
|        |  `0x03` |  pointer to value | Write to the serial port (UART, COM1). | ✅ |
|  `0x33`|  `0x01` |  pointer to buffer | Create an IPv4 packet. | ✅ |
|        |  `0x02` |  pointer to buffer | Create an ICMP packet. | ✅ |
|        |  `0x03` |  pointer to buffer | Create a TCP packet. | ✅ |
|  `0x34`|  `0x01` |  pointer to buffer | Send an IPv4 packet.  | ✅ |
|  `0x35`|  process ID |  pointer to buffer | Socket receive. Blocking op, pops a message form the process' MQ. | ✅  |
|  `0x36`|  `0x01` |  pointer to buffer | Socket send. Pushes a message to the process' MQ. | ✅  |
|  `0x37`| port number (0 for global driver) | __unused__ | Ethernet driver registration / port binding. | ✅ |

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

`system_uptime` holds the number of seconds since boot, derived from the PIT tick counter (100 Hz).

```rust
pub struct SysInfo {
    pub system_name: [u8; 32],
    pub system_user: [u8; 32],
    pub system_path: [u8; 32],
    pub system_version: [u8; 8],
    pub system_path_cluster: u32,
    pub system_uptime: u32,  // seconds since boot
}
```

```c
typedef struct {
    uint8_t  system_name[32];
    uint8_t  system_user[32];
    uint8_t  system_path[32];
    uint8_t  system_version[8];
    uint32_t system_path_cluster;
    uint32_t system_uptime;   /* seconds since boot */
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
    pub year: u16,
}
```

```c
typedef struct {
    uint8_t seconds;
    uint8_t minutes;
    uint8_t hours;
    uint8_t day;
    uint8_t month;
    uint16_t year;
} __attribute__((packed)) RTC_T;
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
} __attribute__((packed)) Entry_T;
```

#### FsckReport (syscall `0x2b`)

```rust
pub struct FsckReport {
    pub errors: u64,
    pub orphan_clusters: u64,
    pub cross_linked: u64,
    pub invalid_entries: u64,
}
```

```c
typedef struct {
    uint64_t errors;
    uint64_t orphan_clusters;
    uint64_t cross_linked;
    uint64_t invalid_entries;
} __attribute__((packed)) FsckReport_T;
```

#### MountInfo (syscall `0x2c`)

Each entry describes one VFS mount point.  The kernel writes up to 8 entries into the caller-supplied array and returns the count.

| Field | Type | Description |
|-------|------|-------------|
| `path` | `uint8_t[32]` | Mount path, **not** NUL-terminated; use `path_len` |
| `path_len` | `uint8_t` | Number of valid bytes in `path` |
| `fs_type` | `uint8_t` | `0`=none, `1`=rootfs, `2`=fat12 |

```rust
pub struct MountInfo {
    pub path: [u8; 32],
    pub path_len: u8,
    pub fs_type: u8,   // 0=none 1=rootfs 2=fat12
}
```

```c
typedef struct {
    uint8_t path[32];
    uint8_t path_len;
    uint8_t fs_type;   /* 0=none, 1=rootfs, 2=fat12 */
} __attribute__((packed)) MountInfo_T;
```
