# Filesystem (VFS / FAT12 / ISO9660)

File name arguments accept either a bare name relative to the current working directory (e.g. `FOO.TXT`) or an absolute VFS path (e.g. `/mnt/fat/FOO.TXT`, `/mnt/tmp/FOO.TXT`, `/mnt/iso/grub/grub.cfg`). Both forms are resolved through the VFS mount table. ISO9660 is mounted read-only at `/mnt/iso`.

`/mnt/tmp` is a FAT16 RAM disk (a sixteenth of the RAM, 126 MiB under QEMU's 2 GiB; FAT12 when that comes to only 2 MiB; empty at every boot but for the kernel's `KERNDBG.LOG`). Every FAT12 syscall below works on it exactly as on the floppy at `/mnt/fat`, which makes it the scratch space for programs booted without a floppy: `/mnt/fat` is mounted only when a FAT12 floppy is found at boot. A bare name goes to whichever of the two volumes the working directory is on. A working directory on neither (`/`, `/mnt`) still sends it to the floppy's root, unlike the kernel shell, where `/` is only the root of the mount table.

Relative names may reach into subdirectories (`GFX/19.IMG`): every component is walked, not just the working directory. Mount prefixes are matched case-insensitively, and names below a FAT12 mount are folded to 8.3 upper case.

## 0x20 (Read file to buffer)

Read a file at the given path and load its whole contents into the buffer. Dispatches to ISO9660 for `/mnt/iso/...` paths.

The kernel is never told the size of the buffer, so the caller must be able to hold the largest file it may meet. Prefer [`0x39`](#0x39-read-part-of-a-file) for anything whose size is not known in advance.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to file name string | pointer to buffer | ✅ |

## 0x21 (Write buffer to file)

Write the buffer into a file (overwrite it) specified by the first argument. The file is created where the path says (in the working directory for a bare name, or in the named subdirectory) if it does not exist. Returns `FileNotFound` when a directory along the path is missing.

Exactly 512 bytes are read from the buffer, so the resulting file is always one sector long. Use [`0x3a`](#0x3a-write-part-of-a-file) to write files of any size or to append.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to string data | pointer to buffer | ✅ |

## 0x22 (Rename file)

Rename the entry specified by `arg1` to `arg2`. Both may be paths into a FAT
subdirectory; a bare destination names the source's own directory. Cross-volume
or cross-directory moves and an existing destination are refused. Jug uses
this operation to publish a verified download while retaining its old copy.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to string data | pointer to string data | ✅ |

## 0x23 (Delete file or empty directory)

Delete the file specified in `arg1`, or with `arg2` = `0x01` the directory. Applicable on an entry in the working directory. The entry's cluster chain is returned to the FAT before the directory entry is marked deleted. A file and a directory of the same name are told apart by `arg2`: each only matches its own kind. A directory is deleted only when it holds nothing but `.` and `..`; its contents are never taken along. Returns `FileNotFound` when nothing was deleted (missing, the wrong kind, or a directory that is not empty).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to string data | `0x00` file, `0x01` empty directory | ✅ |

## 0x24 (Read FAT table)

Read the FAT table and find next (or first) sector of provided cluster.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| cluster No. | pointer to next cluster No. (int64) | ❌ |

## 0x25 (Write FAT cluster)

Write into given cluster such value provided in the `arg2`.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| cluster No. | value | ❌ |

## 0x26 (Insert entry to directory)

Insert an Entry provided via the first argument into the directory with cluster No. specified in the `arg2`.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| cluster No. | pointer to the Entry structure | ❌ |

## 0x27 (Create subdirectory)

Create a subdirectory inside the parent path. Resolves via VFS; ISO9660 paths are rejected (read-only).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to parent directory absolute path | pointer to new subdirectory name | ✅ |

## 0x28 (List FAT12 directory)

List the FAT12 directory at the given cluster.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| cluster No. | pointer to array of entries | ✅ |

## 0x29 (Execute flat binary)

Execute a flat binary executable (`.BIN` usually).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to file name string | pointer to uint64 (PID) | ❌ |

## 0x2a (Execute ELF64 executable)

Execute an ELF64 executable (`.ELF`). Auto-appends `.elf`/`.ELF` if no extension given. Returns the new process PID on success, `0` on failure.

The file is looked up in `/mnt/tmp/jug` first, then in the caller's working directory (on FAT, ISO9660 or the archive), then in `/mnt/tar/bin` and `/mnt/iso/bin`. Jug's downloaded updates therefore take precedence even when the working directory holds a shipped copy. The program is started in the background; it fails when all ten process slots are held by live processes.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to NUL-terminated file name | pointer to NUL-terminated args string (space-delimited; `0` = use file name as sole argv[0]) | ✅ |

## 0x2b (Run FAT12 filesystem check)

Run the FAT12 filesystem check; populates the report struct pointed to by `arg2`.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| *unused* | pointer to `FsckReport_T` | ✅ |

## 0x2c (List VFS mounts)

List VFS mount points. Returns the number of active mounts as a u64. `fs_type`: 

+ `0` = none
+ `1` = rootfs
+ `2` = fat12
+ `3` = iso9660
+ `4` = tar (the boot medium archive at `/mnt/tar`)
+ `5` = memdisk (the RAM disk at `/mnt/tmp`: FAT16, or FAT12 when it is only 2 MiB; [`0x40`](#0x40-size-of-the-filesystem-a-path-is-on) says which).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| *unused* | pointer to array of up to 8 `MountInfo_T` | ✅ |

## 0x2d (List VFS directory)

List a directory by VFS path. Works for both FAT12 and ISO9660. Returns entry count (0–64), or `u64::MAX` (`-1` as `int64_t`) on any error.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to absolute path string | pointer to array of up to 64 `VfsDirEntry_T` | ✅ |

## 0x2e (Change working directory)

Change working directory. Updates `SYSTEM_CONFIG` path and cluster. Verifies the path is an existing directory. ISO9660 paths set cluster to 0. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to absolute path string | `0x00` | ✅ |

## 0x40 (Size of the filesystem a path is on)

Fills an [`FsStat_T`](../type_definitions.md#fsstat-syscall-0x40) with the size of the filesystem the path is on: the whole volume and the bytes still free for files, the mount's type (numbered as [`0x2c`](#0x2c-list-vfs-mounts) numbers it) and the format on the medium (`0` none, `1` fat12, `2` fat16, `3` iso9660, `4` tar). The path is absolute or relative to the working directory. The root is no filesystem: everything is 0. On the read-only mounts the free bytes are 0.

| Mount | `total_bytes` | `free_bytes` |
|-------|---------------|--------------|
| `/` | 0 | 0 |
| `/mnt/fat`, `/mnt/tmp` | the whole volume | the free clusters |
| `/mnt/iso` | the volume space size from the primary volume descriptor | 0 |
| `/mnt/tar` | the archive's length | 0 |

Returns `Ok`; `FileNotFound` when no mount holds the path; `FilesystemError` when the medium cannot be read. A kernel from before this call answers `InvalidSyscall` (`0xff`), which is how a program tells it has no sizes to show.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to NUL-terminated path | pointer to `FsStat_T` | ✅ |

## 0x2f (List scheduler tasks)

List scheduler tasks. Writes up to 10 × 28-byte [`TaskInfo`](../type_definitions.md#taskinfo-syscall-0x2f) entries. Returns the number of entries written.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to output buffer | max entries to write (0 = use default of 10) | ✅ |

## 0x39 (Read part of a file)

Read at most `length` bytes starting `offset` bytes into a file, on FAT12 or ISO9660. Returns the number of bytes read, which is short at the end of the file and `0` when `offset` is past it, or `u64::MAX` on error (missing file, directory, invalid pointer).

Unlike `0x20`, the kernel is told the size of the destination and checks the whole `buffer..buffer+length` range, so a caller can work through a file it could never hold at once. On FAT12 the cluster chain is walked from the start of the file on each call.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to NUL-terminated file name | pointer to [`ReadRange`](../type_definitions.md#readrange-writerange-syscalls-0x39-0x3a) | ✅ |

## 0x3a (Write part of a file)

Write `length` bytes at byte `offset` of a FAT12 file, leaving the bytes before it intact and growing the file and its cluster chain as needed. A missing file is created. A gap between the old end of the file and `offset` reads back as zeros.

Returns the number of bytes written, which is short if the disk fills up, or `u64::MAX` on error. ISO9660 paths are refused.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to NUL-terminated file name | pointer to [`WriteRange`](../type_definitions.md#readrange-writerange-syscalls-0x39-0x3a) | ✅ |
