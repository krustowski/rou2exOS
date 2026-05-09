# Filesystem (VFS / FAT12 / ISO9660)

File name arguments accept either a bare name relative to the current working directory (e.g. `FOO.TXT`) or an absolute VFS path (e.g. `/mnt/fat/FOO.TXT`, `/mnt/iso/grub/grub.cfg`). Both forms are resolved through the VFS mount table. ISO9660 is mounted read-only at `/mnt/iso`.

## 0x20 (Read file to buffer)

Read a file at the given path and load its contents into the buffer. Dispatches to ISO9660 for `/mnt/iso/...` paths.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to file name string | pointer to buffer | ✅ |

## 0x21 (Write buffer to file)

Write the buffer into a file (overwrite it) specified by the first argument. File is created in the current directory if not exists.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to string data | pointer to buffer | ✅ |

## 0x22 (Rename file)

Rename the file specified by its name in `arg1` to value specified in `arg2`.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to string data | pointer to string data | ✅ |

## 0x23 (Delete file)

Delete the file specified in `arg1`. Applicable on a file in the working directory. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to string data | `0x00` | ✅ |

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
+ `3` = iso9660.

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

## 0x2f (List scheduler tasks)

List scheduler tasks. Writes up to 10 × 20-byte `TaskInfo` entries. Returns the number of entries written.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to output buffer | max entries to write (0 = use default of 10) | ✅ |
