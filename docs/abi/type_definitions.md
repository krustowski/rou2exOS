# Type Definitions

## SysInfo

`system_uptime` holds the number of seconds since boot, derived from the PIT tick counter (100 Hz).

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
| `fs_type` | `uint8_t` | `0`=none, `1`=rootfs, `2`=fat12, `3`=iso9660 |

```rust
pub struct MountInfo {
    pub path: [u8; 32],
    pub path_len: u8,
    pub fs_type: u8,   // 0=none 1=rootfs 2=fat12 3=iso9660
}
```

```c
typedef struct {
    uint8_t path[32];
    uint8_t path_len;
    uint8_t fs_type;   /* 0=none, 1=rootfs, 2=fat12, 3=iso9660 */
} __attribute__((packed)) MountInfo_T;
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
