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

#### Video + Audio Output

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x10`|  pointer to string data | string length | Print provided string to terminal. | ✅ |
|  `0x11`|  `0x00` | `0x00` | Clear the screen. | ✅ |
|  `0x12`|  encoded position | encoded color | Write a graphical pixel. | ✅ |
|  `0x13`|  a 320×200 VGA mode-13h palette-indexed buffer | pointer to RGB or default VGA palette | Write a VGA buffer into kernel framebuffer. | ✅ |
|  `0x14`|  reserved (`0x00`) | pointer to `uint64_t` — receives virtual base address | Maps physical VGA graphics RAM (0xA0000–0xAFFFF) into the calling process at virtual 0xA00\_000 with USER+WRITE. On success writes `0xA00_000` into `*arg2`. Idempotent. | ✅ |
|  `0x15`|  video mode | reserved (`0x00`) | Programs VGA hardware registers for the given mode. | ✅ |
|  `0x16`|  pointer to `FBInfo` struct | unused | Get VESA framebuffer geometry. Writes `{ width, height, pitch, bpp }` into the struct pointed to by arg1. Returns `1` if no framebuffer is available, `0` on success. | ✅ |
|  `0x17`|  pointer to 32bpp pixel buffer | `0x00` for no scaling, or `(src_w << 16) \| src_h` | Blit a 32bpp (0x00RRGGBB) buffer to the VESA framebuffer. The kernel handles pitch mismatch. Scaled blit supported via encoded arg2. | ✅ |
|  `0x18`|  pointer to output buffer (`*mut u8`) | buffer capacity in bytes | Copy the kernel's embedded PSF1 glyph data to userland. Returns `char_size` (bytes per glyph = font height), or `0` on error. Glyph `n` occupies bytes `[n*char_size .. (n+1)*char_size]`; bit 7 (MSB) is the leftmost pixel. | ✅ |
|  `0x1a`|  frequency in Hz | length in milliseconds | Play the frequency. | ✅ |
|  `0x1b`|  `0x01`| pointer to the audio file | Play the MIDI audio file. | ✅ |
|  `0x1f`|  `0x00`|  `0x00`| Stop the player. | ✅ |

#### Filesystem (VFS / FAT12 / ISO9660)

File name arguments accept either a bare name relative to the current working directory (e.g. `FOO.TXT`) or an absolute VFS path (e.g. `/mnt/fat/FOO.TXT`, `/mnt/iso/grub/grub.cfg`).  Both forms are resolved through the VFS mount table.  ISO9660 is mounted read-only at `/mnt/iso`.

| Syscall No. | Argument 1 | Argument 2 | Purpose/Command | Implemented |
|-------------|------------|------------|-----------------|-------------|
|  `0x20`|  pointer to file name string | pointer to buffer | Read a file at the given path and load its contents into the buffer. Dispatches to ISO9660 for `/mnt/iso/…` paths. | ✅ |
|  `0x21`|  pointer to string data | pointer to buffer | Write the buffer into a file (overwrite it) specified by the first argument. File is created in the current directory if not exists. | ✅ |
|  `0x22`|  pointer to string data | pointer to string data | Rename the file specified by its name in argument No. 1 to value specified in argument No. 2. | ✅ |
|  `0x23`|  pointer to string data | `0x00` | Delete the file specified in argument No. 1. Applicable on a file in the current directory. | ✅ |
|  `0x24`|  cluster No. | pointer to next cluster No. int64 |  Read the FAT table and find next (or first) sector of provided cluster. | ❌ |
|  `0x25`|  cluster No. | value | Write into given cluster such value provided in the argument No. 2. | ❌ |
|  `0x26`|  cluster No. | pointer to the Entry structure | Insert an Entry provided via the first argument into the directory with Cluster No. specified in the argument No. 2. | ❌ |
|  `0x27`|  pointer to parent directory absolute path | pointer to new subdirectory name | Create a subdirectory inside the parent path. Resolves via VFS; ISO9660 paths are rejected (read-only). | ✅ |
|  `0x28`|  cluster No. | pointer to array of entries | List the FAT12 directory at the given cluster. | ✅ |
|  `0x29`|  pointer to file name string | pointer to uint64 (PID) | Execute a flat binary executable (.BIN usually). | ❌ |
|  `0x2a`|  pointer to file name string | pointer to uint64 (PID) | Execute an ELF64 executable (.ELF). Auto-appends `.elf` if no extension given. | ✅ |
|  `0x2b` | unused | pointer to `FsckReport_T` | Run the FAT12 filesystem check; populates the report struct pointed to by arg2. | ✅ |
|  `0x2c` | unused | pointer to array of up to 8 `MountInfo_T` | List VFS mount points. Returns the number of active mounts as a u64. `fs_type`: `0`=none, `1`=rootfs, `2`=fat12, `3`=iso9660. | ✅ |
|  `0x2d` | pointer to absolute path string | pointer to array of up to 64 `VfsDirEntry_T` | List a directory by VFS path. Works for both FAT12 and ISO9660. Returns entry count (0–64), or `u64::MAX` (`-1` as `int64_t`) on any error. | ✅ |
|  `0x2e` | pointer to absolute path string | `0x00` | Change working directory. Updates `SYSTEM_CONFIG` path and cluster. Verifies the path is an existing directory. ISO9660 paths set cluster to 0. | ✅ |
|  `0x2f` | pointer to output buffer | max entries to write (0 = use default of 10) | List scheduler tasks. Writes up to 10 × 20-byte `TaskInfo` entries. Returns the number of entries written. | ✅ |

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
|  `0x34`|  `0x01` |  pointer to buffer | Send an IPv4 packet (derives frame length from the IP header). | ✅ |
|        |  `0x04` |  pointer to raw Ethernet frame | Send a raw Ethernet frame. Length is derived from the EtherType field (`0x0800` = IPv4, `0x0806` = ARP). | ✅ |
|  `0x35`|  `0x00` = non-blocking, non-zero = blocking |  pointer to buffer | Socket receive. Pops a message from the calling process' MQ. Non-blocking returns `0` immediately if the queue is empty; blocking suspends the process until a frame arrives. | ✅  |
|  `0x36`|  target process PID |  pointer to buffer | Socket send. Copies 512 bytes from the buffer and pushes a message to the target process' MQ, then wakes the target. | ✅  |
|  `0x37`| port number (0 for global driver) | unused | Ethernet driver registration / port binding. | ✅ |
|  `0x38`| pointer to `NetStatus` struct | unused | Query network status (read-only). Writes `{ mac[6], ip[4], drv_active, n_ports, ports[16] }` into the struct. Returns `InvalidInput` on invalid pointer, `Ok` otherwise. | ✅ |

### Syscall Return Codes

| Code (uint64) | Meaning |
|---------------|---------|
| `0x00` | `Okay` |
| `0xfb` | `NotImplemented` |
| `0xfc` | `InvalidInput` |
| `0xfd` | `FilesystemError` |
| `0xfe` | `FileNotFound` |
| `0xff` | `InvalidSyscall` |
