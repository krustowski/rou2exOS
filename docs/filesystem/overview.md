# Overview

## Filesystem Stack

```
Userland (syscalls 0x20–0x2E)
    │
    ▼
fs/vfs — mount table, path dispatch
    ├── fs/fat12    — floppy FAT12 (read/write)
    ├── fs/iso9660  — CD-ROM ISO9660 (read-only)  ┐ behind fs/rofs::RoFs
    └── fs/ustar    — tar archive in RAM (read-only) ┘
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
| `Tar` | ustar archive loaded by GRUB, at `/mnt/usb` |

### Mounts at Boot

Set up by `init::fs::vfs_init()`:

| Path | FsType | Condition |
|------|--------|-----------|
| `/` | `Root` | Always |
| `/mnt/fat` | `Fat12` | Always |
| `/mnt/iso` | `Iso9660` | Only if `Iso9660::probe()` succeeds |
| `/mnt/usb` | `Tar` | Only if GRUB loaded a tar archive as a module |

### Path Resolution

`VfsTable::resolve(path)` returns `(FsType, relative_sub_path)` using **longest-prefix matching**:

1. Iterate all mounts; check if `path` starts with the mount path, compared case-insensitively (FAT folds names to upper case, so a program handing back a path in another case must still hit the mount).
2. Require exact match or that the next character after the prefix is `/`.
3. The mount with the longest matching prefix wins.
4. Returns the sub-path after stripping the mount prefix (and a leading `/`).

Example: path `b"/mnt/fat/SUBDIR/FILE.TXT"` → `(Fat12, b"SUBDIR/FILE.TXT")`.

### Helpers

| Function | Description |
|----------|-------------|
| `try_fat12_absolute(path)` | Returns `Some(rel)` if `path` resolves under the Fat12 mount |
| `try_iso9660_absolute(path)` | Returns `Some(rel)` if `path` resolves under the Iso9660 mount |
| `try_readonly_absolute(path)` | Returns `Some((fs_type, rel))` if `path` resolves under a read-only mount (Iso9660 or Tar) |
| `mount(path, fs_type)` | Add a mount entry |
| `umount(path)` | Remove a mount entry by path |

These are the primary VFS entry points used by syscall handlers in `abi/syscall.rs`. Both wait (bounded spin) for the mount table lock instead of giving up at the first contention: a missed lock used to make an absolute path look relative and be resolved in the wrong directory.

### Syscall Dispatch Pattern

Every filesystem syscall uses the same two-step dispatch:

```
path → try_readonly_absolute(path)
         Some((fs_type, rel)) → RoFs::probe(fs_type)?.resolve(rel)  [read-only]
         None                 → vfs_resolve_fat12(path) → Filesystem::new(&floppy)
```

`RoFs` (`fs/rofs.rs`) is an enum over `Iso9660` and `Tar`; both hand out `IsoEntry`, so a handler treats the two alike. Writes to either are refused.

`vfs_resolve_fat12(path)` strips the `/mnt/fat/` prefix if present, or falls back to the current working directory cluster from `SYSTEM_CONFIG`.

---

## Working Directory

The current working directory is stored in `SYSTEM_CONFIG` (`init/config.rs`) as two fields:

| Field | Type | Description |
|-------|------|-------------|
| `path` | `[u8; 32]` | String representation (e.g. `/mnt/fat/SUBDIR`) |
| `path_cluster` | `u16` | FAT12 cluster for the directory (0 = root, 0 for ISO9660) |

Changed by syscall `0x2E` (chdir) and the shell's `cd`, which validate that the target exists as a directory before updating.

### Path Normalisation (`fs/vfs/path.rs`)

`vfs::normalize_path(cur, arg, out)` builds the absolute path the shell's `cd` and `dir` work with:

- A relative `arg` is joined onto `cur`; an absolute one replaces it.
- Empty components (`//`, trailing `/`) and `.` are dropped; `..` pops a component, and `..` at the root stays at the root.
- The walk is purely textual. Neither filesystem has symlinks, and it is the only way to honour `..` on ISO9660, whose directory lookups skip the `.`/`..` records.
- It returns `None` when the result does not fit in `PATH_MAX` (32 bytes), which is also the size of `SysInfo.system_path`; callers report the error instead of storing a truncated path.

The module depends on nothing else in the kernel so the host unit tests (`tests/unit/path.rs`) can include it directly.

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

---

## Boot Medium Archive (`/mnt/usb`)

Booted from a USB stick, the kernel has no driver to read the medium it came from. Instead, `make build_iso` packs everything in `iso/` except `boot/` into `iso/boot/usb.tar` (ustar format), and `grub.cfg` loads it next to the kernel with `module2 /boot/usb.tar usb`. GRUB reads it through the firmware, so it works on any medium GRUB can boot from.

- **Relocation** (`fs/ustar/module.rs`): GRUB usually places a module just past the kernel image, where the kernel keeps things at fixed physical addresses (process stacks, the userland heap at `0xC00000`, user frames from `0x1000000` to `0x2400000`). `module::adopt()` runs first thing in `kernel_main` and moves the archive to `0x2800000` (or past the boot information, if that is in the way), after checking the memory map says the target is usable RAM. A module already above `0x2400000` is used in place. The kernel's stack, heap and boot page tables live in the linker's `.boot_reserved` NOLOAD section, so the loaded segment covers them and GRUB will not put the module there.
- **Lookup** (`fs/ustar/mod.rs`): a linear scan over the headers, in memory. `IsoEntry::lba` is the index of an entry's 512-byte header (`ROOT` = `u32::MAX` for the root). Names compare case-insensitively. Directories must have their own entries in the archive, which GNU tar writes.
- **Limits**: read-only, a snapshot taken at boot, and the archive stays in RAM for the life of the system.
