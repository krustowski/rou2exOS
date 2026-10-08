# Overview

## Filesystem Stack

```
Userland (syscalls 0x20–0x2E)
    │
    ▼
fs/vfs — mount table, path dispatch
    ├── fs/fat12    — FAT12 (read/write)            behind fs/fatdev::FatDev
    │     ├── on the floppy   (/mnt/fat)
    │     └── on the RAM disk (/mnt/tmp)
    ├── fs/iso9660  — CD-ROM ISO9660 (read-only)  ┐ behind fs/rofs::RoFs
    └── fs/ustar    — tar archive in RAM (read-only) ┘
         │                              │
    fat12/block.rs  memdisk/block.rs  iso9660/block.rs
    (Floppy/ISA DMA) (MemDisk/RAM)    (Atapi/PIO)
         │                              │
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
| `Tar` | ustar archive loaded by GRUB, at `/mnt/tar` |
| `MemDisk` | FAT12 volume in RAM at `/mnt/tmp` |

### Mounts at Boot

Set up by `init::fs::vfs_init()`:

| Path | FsType | Condition |
|------|--------|-----------|
| `/` | `Root` | Always |
| `/mnt/fat` | `Fat12` | Only if `floppy_check_init()` found a FAT12 volume; an empty mount would answer everything below it with a disk error |
| `/mnt/tmp` | `MemDisk` | Always (formatted empty at every boot) |
| `/mnt/iso` | `Iso9660` | Only if `Iso9660::probe()` succeeds |
| `/mnt/tar` | `Tar` | Only if GRUB loaded a tar archive as a module |

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
| `try_fat_absolute(path)` | Returns `Some((fs_type, rel))` if `path` resolves under a writable FAT12 mount (Fat12 or MemDisk) |
| `try_readonly_absolute(path)` | Returns `Some((fs_type, rel))` if `path` resolves under a read-only mount (Iso9660 or Tar) |
| `mount_dir(dir)` | Returns the names directly below `dir` when it is `/` or on the way to a mount (see below) |
| `mount(path, fs_type)` | Add a mount entry |
| `umount(path)` | Remove a mount entry by path |

These are the primary VFS entry points used by syscall handlers in `abi/syscall.rs`. Both wait (bounded spin) for the mount table lock instead of giving up at the first contention: a missed lock used to make an absolute path look relative and be resolved in the wrong directory.

`FsType::name()` gives the name `mount` and `dir` print for a type (`rootfs`, `fat12`, `iso9660`, `tar`, `memdisk`).

### Directories Made of Mounts

`/` is no filesystem of its own, and neither is `/mnt`: they hold nothing but the way to the mount points. `VfsTable::mount_dir(dir)` lists such a directory from the mount table alone. For each mount below `dir` it takes the name one level down, once (`/mnt/fat` and `/mnt/tmp` both give `mnt` below `/`), and returns them in a `MountDir`, each with the type of the mount it is, or `Root` for a directory that only leads to mounts further down:

| `dir` | Result |
|-------|--------|
| `/` | `mnt` (`Root`); always a `MountDir`, even with nothing mounted |
| `/mnt` | `fat` (`Fat12`), `tmp` (`MemDisk`), `iso` (`Iso9660`), `tar` (`Tar`), whichever are mounted |
| `/mnt/tmp`, `/mnt/iso/bin` | `None`: inside a mounted filesystem, which lists itself |
| `/nope`, `/mn` | `None`: leads to no mount |

`dir` is absolute and normalized, and is matched without regard to case, as `resolve` matches. The kernel shell's `cd` accepts such a directory (storing cluster 0), and its `dir` prints the listing; see [The Root Directory](../shell.md#the-root-directory).

### Syscall Dispatch Pattern

Every filesystem syscall uses the same two-step dispatch:

```
path → try_readonly_absolute(path)
         Some((fs_type, rel)) → RoFs::probe(fs_type)?.resolve(rel)  [read-only]
         None                 → vfs_resolve_fat12(path) → Filesystem::new(&dev)
```

`RoFs` (`fs/rofs.rs`) is an enum over `Iso9660` and `Tar`; both hand out `IsoEntry`, so a handler treats the two alike. Writes to either are refused.

`vfs_resolve_fat12(path)` answers `(rel, base_cluster, dev)`. A path under `/mnt/fat` or `/mnt/tmp` is stripped of its mount prefix and starts at that volume's root; anything else is resolved from the working directory's cluster, on the volume the working directory is on (`FatDev::cwd()`). A working directory on no FAT mount (`/`, `/mnt`) still sends such a name to the floppy's root, as it always has; the kernel shell, which goes by `FatDev::cwd_mounted()`, does not.

### `FatDev` (`fs/fatdev.rs`)

The floppy and the RAM disk run the same `fat12::Filesystem`, which is generic over `BlockDevice`. `FatDev` is an enum (`Floppy`, `Mem`) that implements `BlockDevice` by forwarding to one or the other, so every FAT12 code path opens `Filesystem::new(&dev)` and does not care which disk it has.

| Function | Description |
|----------|-------------|
| `FatDev::of(fs_type)` | The device behind a `Fat12` or `MemDisk` mount |
| `FatDev::for_path(abs)` | The volume an absolute path lies on, and the path below the mount |
| `FatDev::for_dir(abs)` | Like `for_path`, but a path under no FAT mount is the floppy's (a bare path under `/` has always named the floppy to the syscalls) |
| `FatDev::cwd()` | The working directory's volume and cluster, by `for_dir`: the syscalls' view |
| `FatDev::cwd_mounted()` | The same, or `None` when the working directory is on no FAT volume (`/`, `/mnt`, the ISO, the archive): the kernel shell's view |

A FAT cluster number alone no longer identifies a directory: cluster 5 on the floppy and cluster 5 on the RAM disk are different places. The volume is always taken from the path next to it; for the working directory that is `SYSTEM_CONFIG.path`, which is why `chdir` (syscall `0x2E`) stores the normalized absolute path.

---

## Working Directory

The current working directory is stored in `SYSTEM_CONFIG` (`init/config.rs`) as two fields:

| Field | Type | Description |
|-------|------|-------------|
| `path` | `[u8; 32]` | String representation (e.g. `/mnt/fat/SUBDIR`) |
| `path_cluster` | `u16` | FAT12 cluster for the directory on the volume `path` lies on (0 = root; 0 for ISO9660, the archive, `/` and `/mnt`) |

Changed by syscall `0x2E` (chdir) and the shell's `cd`, which validate that the target exists as a directory before updating.

### Path Normalisation (`fs/vfs/path.rs`)

`vfs::normalize_path(cur, arg, out)` builds the absolute path the shell's `cd` and `dir` work with:

- A relative `arg` is joined onto `cur`; an absolute one replaces it.
- Empty components (`//`, trailing `/`) and `.` are dropped; `..` pops a component, and `..` at the root stays at the root.
- The walk is purely textual. Neither filesystem has symlinks, and it is the only way to honour `..` on ISO9660, whose directory lookups skip the `.`/`..` records.
- It returns `None` when the result does not fit in `PATH_MAX` (32 bytes), which is also the size of `SysInfo.system_path`; callers report the error instead of storing a truncated path.

The module depends on nothing else in the kernel so the host unit tests (`tests/unit/path.rs`) can include it directly.

---

## RAM Disk (`/mnt/tmp`, `fs/memdisk/`)

`MemDisk` (`memdisk/block.rs`) is a writable `BlockDevice` over a `&'static Mutex<[u8]>`. Each sector is copied with interrupts off, so a task preempted mid-copy cannot leave the lock held. A sector number past the end reads back as zeros and is dropped on write, so a bad cluster number read from a damaged directory cannot index out of bounds.

`memdisk::TMP` is the disk behind `/mnt/tmp`: a 512 KiB (`TMP_SIZE`) static buffer in the kernel's `.bss`. At boot, `vfs_init()` calls `memdisk::format_tmp()`, which lays an empty FAT12 volume onto it with `fat12::format::format()`, and then mounts it. From then on it behaves exactly like the floppy: files, subdirectories, `rename`, `delete`, `write_file_at` and ELF loading from the working directory all work there. Nothing on it survives a reboot.

The first file on it is `KERNDBG.LOG`, the kernel's debug log as it stood at the end of init; the shell's `debug` command rewrites it with the log as it is now. See [Debug Log](../init/overview.md#debug-log).

`fat12::format::format(dev, sectors, label)` is a small, generic mkfs. It writes one reserved sector, two FATs sized to cover the data area, a 224-entry root directory, one sector per cluster, and media byte `0xF8`. The extended boot record carries the `FAT12` type string that `Filesystem::new` looks for. It refuses volumes too small for any data or too large for FAT12 (4085 clusters or more, about 2 MiB at one sector per cluster).

**Size limit.** The buffer counts toward the kernel image, and the whole image has to end below `0x600000`, where every process's page table maps its private frame instead. With 512 KiB, the release kernel ends near `0x372000` and the debug one near `0x3D9000`. The zeroed buffer also lands in the ELF file, because `.gdt`/`.idt` follow `.bss` in the same loaded segment.

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
| `/mnt/tmp` RAM disk size | 512 KiB (1003 one-sector clusters) |

---

## Boot Medium Archive (`/mnt/tar`)

Booted from a USB stick, the kernel has no driver to read the medium it came from. Instead, `make build_iso` packs everything in `iso/` except `boot/` into `iso/boot/usb.tar` (ustar format), and `grub.cfg` loads it next to the kernel with `module2 /boot/usb.tar usb`. GRUB reads it through the firmware, so it works on any medium GRUB can boot from.

- **Relocation** (`fs/ustar/module.rs`): GRUB usually places a module just past the kernel image, where the kernel keeps things at fixed physical addresses (process stacks, the userland heap at `0xC00000`, user frames from `0x1000000` to `0x2400000`). `module::adopt()` runs first thing in `kernel_main` and moves the archive to `0x2800000` (or past the boot information, if that is in the way), after checking the memory map says the target is usable RAM. A module already above `0x2400000` is used in place. The kernel's stack, heap and boot page tables live in the linker's `.boot_reserved` NOLOAD section, so the loaded segment covers them and GRUB will not put the module there.
- **Lookup** (`fs/ustar/mod.rs`): a linear scan over the headers, in memory. `IsoEntry::lba` is the index of an entry's 512-byte header (`ROOT` = `u32::MAX` for the root). Names compare case-insensitively. Directories must have their own entries in the archive, which GNU tar writes.
- **Limits**: read-only, a snapshot taken at boot, and the archive stays in RAM for the life of the system.
