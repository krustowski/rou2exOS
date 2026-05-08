# Overview

## Filesystem Stack

```
Userland (syscalls 0x20–0x2E)
    │
    ▼
fs/vfs          — mount table, path dispatch
    ├── fs/fat12    — floppy FAT12 (read/write)
    └── fs/iso9660  — CD-ROM ISO9660 (read-only)
         │               │
    fat12/block.rs   iso9660/block.rs
    (Floppy/ISA DMA)  (Atapi/PIO)
         │               │
    BlockDevice trait (fs/block.rs)
```

Both filesystems implement the same `BlockDevice` trait. All higher-level logic (directory walks, FAT chains, ISO records) is built on top of that single abstraction.

---

## `BlockDevice` Trait (`fs/block.rs`)

```rust
pub trait BlockDevice {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]);
    fn write_sector(&self, lba: u64, buffer: &[u8; 512]);
}
```

One sector = 512 bytes for FAT12/floppy. ISO9660 uses 2048-byte logical blocks but maps them to the same interface internally. Implementors: `Floppy`, `Atapi` (write is a no-op), `MemDisk`.

---

## VFS (`fs/vfs/mod.rs`)

The VFS is a simple mount table — it does not provide a unified file descriptor layer or inode abstraction. All it does is map path prefixes to filesystem types, and dispatch resolves which filesystem to use for a given absolute path.

### Mount Table

```rust
pub static VFS: Mutex<VfsTable>

pub struct VfsTable {
    mounts: [VfsMount; MAX_MOUNTS],   // MAX_MOUNTS = 8
    count:  usize,
}

pub struct VfsMount {
    path:     [u8; 32],
    path_len: usize,
    fs_type:  FsType,
}
```

`FsType` enum:

| Variant | Meaning |
|---------|---------|
| `None` | Empty / unused slot |
| `Root` | Root mountpoint (`/`) |
| `Fat12` | FAT12 floppy at `/mnt/fat` |
| `Iso9660` | ISO9660 CD-ROM at `/mnt/iso` |

### Mounts at Boot

Set up by `init::fs::vfs_init()`:

| Path | FsType | Condition |
|------|--------|-----------|
| `/` | `Root` | Always |
| `/mnt/fat` | `Fat12` | Always |
| `/mnt/iso` | `Iso9660` | Only if `Iso9660::probe()` succeeds |

### Path Resolution

`VfsTable::resolve(path)` returns `(FsType, relative_sub_path)` using **longest-prefix matching**:

1. Iterate all mounts; check if `path` starts with the mount path.
2. Require exact match or that the next character after the prefix is `/`.
3. The mount with the longest matching prefix wins.
4. Returns the sub-path after stripping the mount prefix (and a leading `/`).

Example: path `b"/mnt/fat/SUBDIR/FILE.TXT"` → `(Fat12, b"SUBDIR/FILE.TXT")`.

### Helpers

| Function | Description |
|----------|-------------|
| `try_fat12_absolute(path)` | Returns `Some(rel)` if `path` resolves under the Fat12 mount |
| `try_iso9660_absolute(path)` | Returns `Some(rel)` if `path` resolves under the Iso9660 mount |
| `mount(path, fs_type)` | Add a mount entry |
| `umount(path)` | Remove a mount entry by path |

These are the primary VFS entry points used by syscall handlers in `abi/syscall.rs`.

### Syscall Dispatch Pattern

Every filesystem syscall uses the same two-step dispatch:

```
path → try_iso9660_absolute(path)
         Some(rel) → Iso9660::probe()?.resolve(rel)  [read-only]
         None      → vfs_resolve_fat12(path) → Filesystem::new(&floppy)
```

`vfs_resolve_fat12(path)` strips the `/mnt/fat/` prefix if present, or falls back to the current working directory cluster from `SYSTEM_CONFIG`.

---

## Working Directory

The current working directory is stored in `SYSTEM_CONFIG` (`init/config.rs`) as two fields:

| Field | Type | Description |
|-------|------|-------------|
| `path` | `[u8; 32]` | String representation (e.g. `/mnt/fat/SUBDIR`) |
| `path_cluster` | `u16` | FAT12 cluster for the directory (0 = root, 0 for ISO9660) |

Changed by syscall `0x2E` (chdir), which validates that the target exists as a directory before updating.

---

## MemDisk (`fs/memdisk/block.rs`)

`MemDisk` wraps a `&'static mut [u8]` as a `BlockDevice`. `read_sector` copies 512 bytes from the slice. `write_sector` is a no-op. Intended for in-memory disk images but not currently wired into any live code path.

---

## Limits

| Resource | Value |
|----------|-------|
| Max VFS mounts | 8 |
| Max mount path length | 31 bytes |
| FAT12 sector size | 512 bytes |
| ISO9660 block size | 2048 bytes |
| Max directory entries returned (syscall 0x28) | 32 |
| Max directory entries returned (syscall 0x2D) | 64 |
| Max VFS mounts listed (syscall 0x2C) | 8 × 34-byte entries |
