# Type Definitions

## SysInfo

`system_uptime` holds the number of seconds since boot, derived from the PIT tick counter (`TICKS_PER_SECOND`, currently 1000 Hz). `system_path` holds the working directory and is NUL-terminated when shorter than 32 bytes; a working directory can never be longer than 32 bytes (`PATH_MAX`).

```rust
pub struct SysInfo {
    pub system_name: [u8; 32],
    pub system_user: [u8; 32],
    pub system_path: [u8; 32],
    pub system_version: [u8; 8],
    pub system_path_cluster: u32,
    pub system_uptime: u32,  // seconds since boot
    pub ip_addr: [u8; 4],
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
    uint8_t  ip_addr[4];
} __attribute__((packed)) SysInfo_T;
```

## RTC

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

## Entry (FAT12)

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

## FsckReport (syscall `0x2b`)

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

## MountInfo (syscall `0x2c`)

Each entry describes one VFS mount point.  The kernel writes up to 8 entries into the caller-supplied array and returns the count.

| Field | Type | Description |
|-------|------|-------------|
| `path` | `uint8_t[32]` | Mount path, **not** NUL-terminated; use `path_len` |
| `path_len` | `uint8_t` | Number of valid bytes in `path` |
| `fs_type` | `uint8_t` | `0`=none, `1`=rootfs, `2`=fat12, `3`=iso9660, `4`=tar, `5`=memdisk |

```rust
pub struct MountInfo {
    pub path: [u8; 32],
    pub path_len: u8,
    pub fs_type: u8,   // 0=none 1=rootfs 2=fat12 3=iso9660 4=tar 5=memdisk
}
```

```c
typedef struct {
    uint8_t path[32];
    uint8_t path_len;
    uint8_t fs_type;   /* 0=none, 1=rootfs, 2=fat12, 3=iso9660, 4=tar, 5=memdisk */
} __attribute__((packed)) MountInfo_T;
```

`5`=memdisk is the RAM disk at `/mnt/tmp` whatever its format; `FsStat.format` tells FAT16 from FAT12.

## FsStat (syscall `0x40`)

The size of the filesystem a path is on. 24 bytes, packed.

| Field | Type | Description |
|-------|------|-------------|
| `total_bytes` | `uint64_t` | The whole volume; 0 for the root |
| `free_bytes` | `uint64_t` | Bytes free for files; 0 on the read-only mounts |
| `fs_type` | `uint8_t` | The mount's type, numbered as `MountInfo.fs_type` |
| `format` | `uint8_t` | The format on the medium: `0`=none, `1`=fat12, `2`=fat16, `3`=iso9660, `4`=tar |
| `reserved` | `uint8_t[6]` | 0 |

```rust
#[repr(C, packed)]
pub struct FsStat {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub fs_type: u8,
    pub format: u8,    // 0=none 1=fat12 2=fat16 3=iso9660 4=tar
    pub reserved: [u8; 6],
}
```

```c
typedef struct {
    uint64_t total_bytes;
    uint64_t free_bytes;
    uint8_t fs_type;   /* FS_TYPE_* */
    uint8_t format;    /* FS_FORMAT_*: 0=none, 1=fat12, 2=fat16, 3=iso9660, 4=tar */
    uint8_t reserved[6];
} __attribute__((packed)) FsStat_T;
```

## FBInfo (syscall `0x16`)

Describes the active VESA framebuffer geometry.  All fields are in pixels or bytes.

| Field | Type | Description |
|-------|------|-------------|
| `width` | `uint32_t` | Framebuffer width in pixels |
| `height` | `uint32_t` | Framebuffer height in pixels |
| `pitch` | `uint32_t` | Bytes per scanline (may be larger than `width × bpp/8`) |
| `bpp` | `uint32_t` | Bits per pixel |

```rust
pub struct FBInfo {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
}
```

```c
typedef struct {
    uint32_t width;
    uint32_t height;
    uint32_t pitch;
    uint32_t bpp;
} __attribute__((packed)) FBInfo_T;
```

## FBCaptureInfo (syscall `0x1d`)

Optional metadata for the RGB24 capture, passed in `RCX` when bit 63 of argument 2 is set (see [`0x1d`](syscalls/video_audio.md#metadata-bit-63-of-argument-2)). Written with an unaligned store.

| Field | Type | Description |
|-------|------|-------------|
| `frame_id` | `uint64_t` | In: the snapshot the caller already has (`0` = always copy). Out: the snapshot served, `0` for a capture read from VRAM |
| `timestamp_ms` | `uint64_t` | Out: when the snapshot was published, or the uptime after a VRAM read, in milliseconds |
| `flags` | `uint32_t` | Out: `1` (`FB_CAPTURE_INFO_SNAPSHOT`) when served from a stable snapshot in RAM, else `0` |
| `reserved` | `uint32_t` | Zero |

```rust
#[repr(C)]
pub struct FrameCaptureInfo {
    pub frame_id: u64,
    pub timestamp_ms: u64,
    pub flags: u32,
    pub reserved: u32,
}
```

```c
typedef struct {
    uint64_t frame_id;
    uint64_t timestamp_ms;
    uint32_t flags;
    uint32_t reserved;
} FBCaptureInfo_T;
```

## NetStatus (syscall `0x38`)

Describes the current network driver state.  All fields are filled by the kernel from `SYSTEM_CONFIG` and the port-binding registry.

| Field | Type | Description |
|-------|------|-------------|
| `mac` | `uint8_t[6]` | Ethernet MAC address |
| `ip` | `uint8_t[4]` | IPv4 address |
| `drv_active` | `uint8_t` | `1` if an Ethernet driver process is registered, `0` otherwise |
| `n_ports` | `uint8_t` | Number of bound TCP ports |
| `ports` | `uint16_t[16]` | Array of bound TCP port numbers (`n_ports` entries valid) |

```rust
pub struct NetStatus {
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub drv_active: u8,
    pub n_ports: u8,
    pub ports: [u16; 16],
}
```

```c
typedef struct {
    uint8_t  mac[6];
    uint8_t  ip[4];
    uint8_t  drv_active;
    uint8_t  n_ports;
    uint16_t ports[16];
} __attribute__((packed)) NetStatus_T;
```

## NetConfig (syscall `0x3d`)

The network configuration the global Ethernet driver publishes; 30 bytes, packed. Unset fields are zero.

| Field | Type | Description |
|-------|------|-------------|
| `ip` | `uint8_t[4]` | IPv4 address |
| `netmask` | `uint8_t[4]` | Netmask of the local network |
| `gateway` | `uint8_t[4]` | Gateway for destinations off the local network |
| `dns` | `uint8_t[4]` | DNS server |
| `mac` | `uint8_t[6]` | The network card's MAC (read only) |
| `gateway_mac` | `uint8_t[6]` | The gateway's MAC, as the driver resolved it by ARP |
| `source` | `uint8_t` | `0` not set, `1` static, `2` DHCP, `3` fallback (no DHCP answer yet) |
| `_reserved` | `uint8_t` | Zero |

```rust
pub struct NetConfig {
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns: [u8; 4],
    pub mac: [u8; 6],
    pub gateway_mac: [u8; 6],
    pub source: u8,
    pub _reserved: u8,
}
```

```c
typedef struct {
    uint8_t ip[4];
    uint8_t netmask[4];
    uint8_t gateway[4];
    uint8_t dns[4];
    uint8_t mac[6];
    uint8_t gateway_mac[6];
    uint8_t source;
    uint8_t _reserved;
} __attribute__((packed)) NetConfig_T;
```

## VfsDirEntry (syscall `0x2d`)

Each entry describes one item in a directory.  The kernel writes up to 64 entries and returns the count, or `u64::MAX` (`-1` as `int64_t`) on error.  `name` is **not** NUL-terminated; use `name_len`.

| Field | Type | Description |
|-------|------|-------------|
| `name` | `uint8_t[32]` | Entry name, lowercase, **not** NUL-terminated |
| `name_len` | `uint8_t` | Number of valid bytes in `name` |
| `is_dir` | `uint8_t` | `1` if directory, `0` if file |
| `size` | `uint32_t` | File size in bytes (0 for directories) |

```rust
pub struct VfsDirEntry {
    pub name: [u8; 32],
    pub name_len: u8,
    pub is_dir: u8,
    pub size: u32,
}
```

```c
typedef struct {
    uint8_t  name[32];
    uint8_t  name_len;
    uint8_t  is_dir;
    uint32_t size;
} __attribute__((packed)) VfsDirEntry_T;
```

## TaskInfo (syscall `0x2f`)

One entry per live scheduler slot, 28 bytes each.

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| 0 | `id` | `uint8_t` | PID (the value `0x3b` and the shell's `kill` take) |
| 1 | `mode` | `uint8_t` | `0` = kernel, `1` = user |
| 2 | `status` | `uint8_t` | `0` = Ready, `1` = Running, `2` = Idle, `3` = Blocked, `4` = Crashed, `5` = Dead |
| 3 | `_pad` | `uint8_t` | |
| 4 | `name` | `uint8_t[16]` | Process name, space-padded |
| 20 | `rip` | `uint64_t` | Where the process was last interrupted, read from its saved frame; `0` for the calling process. A process that keeps reporting the same `rip` is spinning there. |

```c
typedef struct {
    uint8_t  id;
    uint8_t  mode;
    uint8_t  status;
    uint8_t  _pad;
    uint8_t  name[16];
    uint64_t rip;
} __attribute__((packed)) TaskInfo_T;
```

## ReadRange, WriteRange (syscalls `0x39`, `0x3a`)

Passed by pointer so that the syscall keeps its two-argument shape. Fields are read unaligned.

| Field | Type | Description |
|-------|------|-------------|
| `buffer` | `uint64_t` | Address of the destination (read) or source (write); `buffer..buffer+length` must lie wholly in `0x400_000..0xA00_000` (image and initial stacks) or wholly in the user heap `0xC00_000..0x1000_000` |
| `offset` | `uint64_t` | Byte offset into the file |
| `length` | `uint64_t` | Number of bytes to transfer |

```rust
#[repr(C)]
struct ReadRange {
    buffer: u64,
    offset: u64,
    length: u64,
}
```

```c
typedef struct {
    uint64_t buffer;
    uint64_t offset;
    uint64_t length;
} __attribute__((packed)) ReadRange_T, WriteRange_T;
```

## MemInfo (syscall `0x3c`)

Every field is a `uint64_t` byte count unless it says otherwise. Written unaligned.

| Field | Type | Description |
|-------|------|-------------|
| `version` | `uint64_t` | `1` |
| `total_ram` | `uint64_t` | Usable RAM the boot loader reported |
| `heap_start`, `heap_size` | `uint64_t` | The user heap: `0xC00_000`, 4 MiB |
| `heap_used`, `heap_free` | `uint64_t` | Payload bytes in used and in free blocks |
| `heap_largest_free` | `uint64_t` | The largest block one allocation can still get |
| `heap_blocks`, `heap_free_blocks` | `uint64_t` | Blocks in the heap, and how many are free |
| `heap_by_slot` | `uint64_t[17]` | Used bytes by process slot; `[16]` is untagged (kernel staging, blocks allocated with no owner), and slots 16 and up |
| `frame_base`, `frame_size` | `uint64_t` | Slot *n*'s private frame is physical `frame_base + n * frame_size` |
| `frame_virt` | `uint64_t` | Where each process sees its frame: `0x600_000` |
| `slots` | `uint64_t` | Process slots (32; only the first 16 are in `slot_task`) |
| `slot_task` | `uint8_t[16]` | The task id (as `0x2f` reports it) in each of slots 0–15, `0xFF` when free |

```c
typedef struct {
    uint64_t version;
    uint64_t total_ram;
    uint64_t heap_start;
    uint64_t heap_size;
    uint64_t heap_used;
    uint64_t heap_free;
    uint64_t heap_largest_free;
    uint64_t heap_blocks;
    uint64_t heap_free_blocks;
    uint64_t heap_by_slot[17];
    uint64_t frame_base;
    uint64_t frame_size;
    uint64_t frame_virt;
    uint64_t slots;
    uint8_t  slot_task[16];
} __attribute__((packed)) MemInfo_T;
```

## MemInfo2 (syscall `0x3c`, version 2)

Asked for with `2` in Argument 2. The same fields as `MemInfo`, with room for 32 slots; `version` is `2`. 400 bytes.

| Field | Type | Description |
|-------|------|-------------|
| `heap_by_slot` | `uint64_t[33]` | Used bytes by process slot; `[32]` is untagged |
| `slots` | `uint64_t` | Process slots (32) |
| `slot_task` | `uint8_t[32]` | The task id in each slot, `0xFF` when free |

```c
typedef struct {
    uint64_t version;
    uint64_t total_ram;
    uint64_t heap_start;
    uint64_t heap_size;
    uint64_t heap_used;
    uint64_t heap_free;
    uint64_t heap_largest_free;
    uint64_t heap_blocks;
    uint64_t heap_free_blocks;
    uint64_t heap_by_slot[33];
    uint64_t frame_base;
    uint64_t frame_size;
    uint64_t frame_virt;
    uint64_t slots;
    uint8_t  slot_task[32];
} __attribute__((packed)) MemInfo2_T;
```
