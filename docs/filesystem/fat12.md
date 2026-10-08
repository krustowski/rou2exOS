# FAT12 and FAT16

FAT is the read/write filesystem: FAT12 on the 1.44 MB floppy, FAT16 on the RAM disk at `/mnt/tmp` (FAT12 too when the RAM disk is only 2 MiB; see [RAM Disk](overview.md#ram-disk-mnttmp-fsmemdisk)). One driver, `fs/fat12`, reads and writes both, with clusters of one sector or of several. `FatDev` selects the device from the mount or working directory, so directory clusters are interpreted on the correct volume. It is accessible as:

- Absolute paths under `/mnt/fat/` (mounted only when a FAT12 floppy is found at boot) or `/mnt/tmp/`
- Bare filenames relative to the current working directory (cluster stored in `SYSTEM_CONFIG`)

The module keeps the name `fat12` from when that was all it read.

---

## Hardware: Floppy Controller (`fs/fat12/block.rs`)

`Floppy` implements `BlockDevice` using direct ISA DMA and FDC I/O port programming.

### DMA Setup (`Floppy::init`)

Initialises ISA DMA channel 2. Single-sector fallback reads transfer one sector into the static `.dma` buffer; track reads program DMA directly to a cache slot. The single-sector setup programs ISA DMA channel 2 to transfer one sector (512 bytes) from the floppy controller into the static `DMA: [u8; 512]` buffer:

```
port 0x0A ← 0x06   Mask channels 2+0
port 0x0C ← 0xFF   Reset flip-flop
port 0x04 ← addr_lo, addr_hi   DMA buffer address
port 0x05 ← 511 lo, 511 hi     Transfer count − 1
port 0x81 ← page               High byte of DMA address
port 0x0A ← 0x02               Unmask channel 2
```

The `DMA` buffer is placed in a `.dma` link section so its physical address is known at build time.

### Read Sector (`read_sector`)

1. Convert LBA → CHS: `C = LBA / (18 × 2)`, `H = (LBA % 36) / 18`, `S = (LBA % 18) + 1`.
2. Set DMA read mode (`port 0x0B ← 0x56`): single transfer, address increment, read, channel 2.
3. Send FDC READ DATA command (0x46) with head/cylinder/head/sector/byte-size/18/GAP3/DTL.
4. Poll the MSR until the result phase opens, then read the seven result bytes and validate status. Seek/recalibrate completion uses Sense Interrupt separately.
5. `copy_nonoverlapping(DMA, buffer, 512)`.

The floppy disk geometry assumed throughout: 80 cylinders, 2 heads, 18 sectors/track = 2880 sectors × 512 bytes = 1.44 MB.

### Track Cache

Reads do not go to the drive one sector at a time. `BlockDevice::read_sector` serves the sector from an 8-slot, LRU **track cache** (`CACHE` in `block.rs`):

- A miss reads the whole track (18 sectors, 9216 bytes) with one READ DATA command, straight into the cache slot. Each slot is 16 KiB-aligned so an ISA DMA transfer never crosses a 64 KiB physical boundary.
- Several slots are needed because FAT12 alternates between the FAT (track 0), the root directory (track 1) and the data area.
- The cache is flushed when the FDC's disk-change line (DIR bit 7) is set, and a written sector's track is invalidated before the write.
- If the drive rejects a multi-sector transfer (after one recalibrate and retry), the cache stops asking and falls back to single-sector reads through the `.dma` buffer described above.

Every request runs with interrupts disabled: the FDC is driven by a multi-byte command handshake, and a preemption between two command bytes would leave the controller mid-command for the next task. Every handshake step has a spin budget, so a missing or unresponsive drive fails a read rather than hanging.

### Write Sector (`write_sector`)

1. Seeks to the target cylinder/head using FDC SEEK command (0x0F).
2. Reprograms DMA channel 2 for memory→device transfer (mode 0x58) with the data copied to `DMA_BUFFER_ADDR = 0x1000`.
3. Sends FDC WRITE DATA command (0x45).
4. Waits for IRQ6.

### FDC Ports

| Port | Register |
|------|---------|
| `0x3F2` | Digital Output Register (DOR) — motor, drive select |
| `0x3F4` | Main Status Register (MSR) — ready/busy flags |
| `0x3F5` | Data FIFO — command/result bytes |

---

## Filesystem Structure (`fs/fat12/fs.rs`)

`Filesystem<D: BlockDevice>` holds all computed layout values derived from the boot sector:

| Field | Description |
|-------|-------------|
| `device` | Reference to the underlying `BlockDevice` |
| `boot_sector` | Parsed copy of the BPB (sector 0) |
| `fat_start_lba` | LBA of FAT table (= `reserved_sectors`) |
| `root_dir_start_lba` | LBA of root directory region |
| `data_start_lba` | LBA of first data cluster |
| `sectors_per_cluster` | Sectors per cluster from BPB |
| `kind` | `FatKind::Fat12` or `FatKind::Fat16` |
| `cluster_count` | Clusters in the data area, numbered from 2; also the bound on every walk along a chain, so a FAT that loops ends the walk |
| `total_sectors` | Sectors on the whole volume, from the 16-bit count or, when that is 0, the 32-bit one |

### Boot Sector / BPB (`fs/fat12/entry.rs`)

`BootSector` is `#[repr(C, packed)]`, read directly from sector 0:

| Offset | Field | Description |
|--------|-------|-------------|
| 0 | `jmp` | 3-byte jump instruction |
| 3 | `oem` | OEM string (8 bytes) |
| 11 | `bytes_per_sector` | Always 512 |
| 13 | `sectors_per_cluster` | Sectors per allocation unit |
| 14 | `reserved_sectors` | Sectors before the FAT (usually 1) |
| 16 | `fat_count` | Number of FAT copies (usually 2) |
| 17 | `root_entry_count` | Max root directory entries (usually 224) |
| 19 | `total_sectors_16` | Total sectors on disk; 0 when there are more than 65535 |
| 22 | `fat_size_16` | Sectors per FAT copy (usually 9) |
| 32 | `total_sectors_32` | Total sectors when `total_sectors_16` is 0 |

Detection: `Filesystem::new` scans the boot sector for the 5-byte string `"FAT12"` or `"FAT16"`. If neither is there, it returns `Err`. Which of the two the volume is follows from its cluster count, as the FAT specification has it, not from the string: fewer than 4085 clusters is FAT12 (`FAT12_MAX_CLUSTERS`), fewer than 65525 FAT16 (`FAT16_MAX_CLUSTERS`).

### Layout Arithmetic

```
fat_start_lba      = reserved_sectors
root_dir_sectors   = ceil(root_entry_count × 32 / 512)
root_dir_start_lba = fat_start_lba + (fat_count × fat_size_16)
data_start_lba     = root_dir_start_lba + root_dir_sectors
cluster_to_lba(n)  = data_start_lba + (n − 2) × sectors_per_cluster
```

---

## Directory Entries (`fs/fat12/entry.rs`)

Each directory entry is 32 bytes (`#[repr(C, packed)]`):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 8 | `name` | Filename, space-padded, uppercase |
| 8 | 3 | `ext` | Extension, space-padded, uppercase |
| 11 | 1 | `attr` | Attribute flags (see below) |
| 12 | 1 | `reserved` | |
| 13 | 1 | `create_time_tenths` | |
| 14 | 2 | `create_time` | |
| 16 | 2 | `create_date` | |
| 18 | 2 | `last_access_date` | |
| 20 | 2 | `high_cluster` | High 16 bits of cluster (unused in FAT12) |
| 22 | 2 | `write_time` | |
| 24 | 2 | `write_date` | |
| 26 | 2 | `start_cluster` | First cluster of file data |
| 28 | 4 | `file_size` | File size in bytes (0 for directories) |

### Attribute Byte Flags

| Bit | Value | Meaning |
|-----|-------|---------|
| 0 | 0x01 | Read-only |
| 1 | 0x02 | Hidden |
| 2 | 0x04 | System |
| 3 | 0x08 | Volume label |
| 4 | 0x10 | Directory |
| 5 | 0x20 | Archive |

### Special First-Byte Values

| Value | Meaning |
|-------|---------|
| `0x00` | Free; by the FAT spec the end of the directory, but lookups keep scanning (see [Directory Search](#directory-search)) |
| `0xE5` | Deleted entry (slot is reusable) |
| `0xFF` | Unused (treated as invalid) |

---

## FAT Table Encoding (`fs/fat12/table.rs`, `fs/fat12/fs.rs`)

FAT16 keeps each entry in two bytes, little-endian, at byte offset `N * 2`; an entry never straddles two sectors.

FAT12 encodes each cluster entry in 12 bits. Two cluster numbers share 3 bytes, packed as follows:

For an even cluster N at byte offset `fat_offset = (N * 3) / 2`:
```
value = byte[fat_offset] | ((byte[fat_offset+1] & 0x0F) << 8)
```

For an odd cluster N:
```
value = (byte[fat_offset] >> 4) | (byte[fat_offset+1] << 4)
```

### Special Cluster Values

| Range | Meaning |
|-------|---------|
| `0x000` | Free cluster |
| `0x001` | Reserved |
| `0x002–0xFEF` | Valid data cluster, value = next cluster in chain |
| `0xFF0–0xFF6` | Reserved |
| `0xFF7` | Bad sector |
| `0xFF8–0xFFF` | End of chain (EOF) |

FAT16 has the same values sixteen bits wide: `0xFFF7` bad, `0xFFF8–0xFFFF` end of chain, data clusters up to `0xFFF6`.

### One numbering for both

`Filesystem::read_fat_entry` answers in FAT16's numbering on either kind of volume: a FAT12 value from `0xFF7` up is widened by four bits, so FAT12's bad cluster and ends of chain come back as `0xFFF7` and `0xFFF8–0xFFFF`. Everything that walks a chain compares against `fs::CHAIN_END` (`0xFFF8`), or asks `fs::is_data_cluster(c)` (2 up to `CHAIN_END`), instead of FAT12's `0xFF8`: a FAT16 volume has data clusters numbered past `0xFF8`. A chain's last entry is written as `0xFFFF`, of which FAT12 keeps `0xFFF`.

`FatTable` reads all 9 FAT sectors (4608 bytes) into a single `[u8; 4608]` for batch inspection; it and `fsck` are for the floppy, and FAT12, only. `Filesystem::read_fat_entry` reads individual sectors on demand, handling the FAT12 cross-sector case (an entry starting at `fat_offset % 512 == 511`).

`write_fat_entry` updates **every** FAT copy (`fat_count`), not just the first, so a checker comparing them does not report the volume as damaged. A FAT12 entry that straddles two sectors is written to both. Its second byte used to be written to the first byte of the *same* sector, over another cluster's entry, and its own high bits never at all; on the floppy that was clusters 341, 682, 1023 and every 341 or 342 after.

### Allocation

`allocate_cluster_near(c)` claims the first free cluster after `c`, wrapping round to cluster 2, and marks it as the end of a chain; `allocate_cluster()` searches from the start. A chain being written (`write_file`, `write_file_at`, a directory growing) asks near its own last cluster, so filling a file is one pass over the FAT rather than a scan from the front for every cluster: on a large RAM disk the front can be tens of thousands of clusters long. The scan reads each FAT sector once (`find_in_fat`) and returns `0` when the FAT is full. `free_clusters()` counts the free entries the same way, for the mount sizes of syscall [`0x40`](../abi/syscalls/filesystem.md#0x40-size-of-the-filesystem-a-path-is-on).

A cluster that is handed out keeps what it held: a deleted file's bytes, or on the RAM disk what the memory held at boot, `format` clearing only the FAT and the root. So `zero_cluster` empties every cluster that becomes a directory or is added by `write_file_at`, whose gaps read back as zeros.

---

## Operations

### Read File (`read_file`, `read_range`)

`read_file(start_cluster, buf)` follows the cluster chain and reads every sector of every cluster into `buf`, whole sectors, stopping at the end of the chain or when the buffer is full; a file longer than the buffer is cut short at its end. It used to read one sector per cluster, which is a whole cluster on the floppy alone, and to loop forever once the buffer was full, so the shell's `read` hung on any file over 4 KiB.

`read_range(entry, offset, length, dst)` copies exactly `length` bytes from `offset` into the file, skipping the clusters before it, and returns how many it copied (short at the end of the file). It is what syscalls `0x20` and `0x39` and the ELF loader read with, so none of them copies past the file's size.

### Write File (`write_file`)

1. If a file with the same name exists: `free_cluster_chain` its old clusters, mark its directory entry `0xE5`.
2. Allocate as many clusters as the data takes, a cluster's worth at a time (not a sector's: a volume of larger clusters got a chain of empty ones), each near the last; chain them with `write_fat_entry` and end the chain.
3. Write each 512-byte sector of `data` into them.
4. `write_dir_entry()` adds the 32-byte entry (name, attr `0x20`, cluster, file_size) through `insert_directory_entry`, at the first free slot (`0x00` or `0xE5`) of the directory: anywhere in the root, or anywhere along a subdirectory's chain, which grows by an emptied cluster when it is full. Only the root's first sector used to be looked at, so a root with sixteen files took no more.

### Write at Offset (`write_file_at`)

The write half of a ranged read, exposed as syscall `0x3a`. Writes `data` at byte `offset` without touching the bytes before it:

1. A file that does not exist gets an entry and one cluster; an entry with no cluster gets one.
2. The cluster chain is walked to the cluster holding `offset`, allocating and linking new clusters (`chain_next_or_grow`) when the file is shorter. A gap reads back as zeros.
3. Data is copied a sector at a time; a sector the write starts or ends inside is read first so the surrounding bytes survive.
4. The directory entry's size is updated last. If the disk fills up, the write stops and the size reflects what was actually written.

### Delete File (`delete_file`)

Finds the entry by its full 8.3 name (name *and* extension, so `THEM.LOG` never matches a directory `THEM`), skips directories, returns the cluster chain to the FAT, then marks the first byte of the entry `0xE5`. Returns `false` when there was no such file.

### Directory Search

Lookups (`find_entry`, `for_each_entry`, `resolve_path_from`) walk the whole directory: every sector of the root directory, and the full cluster chain of a subdirectory. On a floppy one cluster is one sector (16 entries), so stopping at the first cluster hid everything past the 16th entry, and `write_file` would then add a duplicate. The walk is bounded by the number of clusters on the volume, so a FAT that loops ends the search. A free slot (`0x00`) is skipped rather than treated as the end, because mtools can leave entries after one.

`resolve_path_from(base, path)` resolves a multi-component relative path one name at a time, stopping at the first match in each directory.

### Rename File (`rename_file`)

Overwrites the 11 name bytes in the directory entry. Does not change cluster chain or file data.

### Create Subdirectory (`create_subdirectory`)

1. `allocate_cluster()` for the new directory.
2. Write a directory entry with `attr = 0x10`, `file_size = 0`.
3. Zero every sector of the cluster: what was there before would read as entries.
4. Write `.` (self-pointer) and `..` (parent-pointer) entries into the new cluster.
5. Mark the new cluster as end-of-chain in the FAT.

### Directory Traversal (`for_each_entry`)

Iterates all 32-byte entries in a directory, calling a closure for each:
- `dir_cluster == 0`: reads the fixed root directory region (`root_dir_start_lba`, `root_entry_count` entries).
- `dir_cluster > 0`: follows the FAT cluster chain for subdirectories.

### Path Lookup (`find_entry`, `resolve_path_from`)

`find_entry(cluster, name83)` calls `for_each_entry` and matches all 11 bytes (name + ext).

`resolve_path_from(start_cluster, path)` splits `path` on `/` and calls `find_entry` for each component, following directory cluster chains. Returns `None` if any component is missing. An empty path returns a synthetic directory entry for `start_cluster` itself.

---

## Filename Format (`fat83`)

`fat83(component)` converts a file name component (e.g. `b"file.txt"`) to the 11-byte FAT 8.3 format:

- Split at the last `.`; left side = name (max 8), right side = extension (max 3).
- Uppercase all bytes.
- Space-pad to `[u8; 11]`.

Example: `b"SH.ELF"` → `b"SH      ELF"`.

---

## Filesystem Check (`fs/fat12/check.rs`)

`run_check()` → `CheckReport { errors, orphan_clusters, cross_linked, invalid_entries }`:

1. `FatTable::load()` reads all 9 FAT sectors into a contiguous buffer.
2. `scan_directory(0, ...)` recursively visits every directory and file from the root (max depth 64):
   - Directories: recurse.
   - Files: `validate_chain` marks each cluster as visited in a `[bool; 4096]` bitmap, detects cross-linked clusters (already-visited cluster reused by a different file).
3. After scanning, counts orphan clusters: FAT entries that are non-zero (allocated) but were never visited by `scan_directory`.

Exposed via syscall `0x2B`. Returns four `u64` values written to a userland `FsckReport_T` struct.

---

## Limits

| Resource | Value |
|----------|-------|
| Disk size | 1.44 MB (80 cyl × 2 heads × 18 sectors × 512 B) |
| Max root directory entries | 224 (from BPB; not expandable) |
| Sector size | 512 bytes |
| Max cluster index | 4084 (FAT12) |
| FAT copy sectors | 9 sectors per copy, 2 copies |
| Max file size | Limited by available clusters × 512 B |
| Max directory depth (fsck) | 64 |

---

## Formatting (`fs/fat12/format.rs`)

`format(dev, sectors, label)` lays an empty volume onto a block device, choosing the kind from the size:

| Size | Kind | Cluster | Root entries |
|------|------|---------|--------------|
| up to about 2 MiB (fewer than 4085 clusters at one sector each) | FAT12 | 512 B | 224 |
| from there to 32 MiB | FAT16 | 512 B | 512 |
| larger | FAT16 | the smallest power-of-two number of sectors, up to 64 (32 KiB), that keeps the count below 65525: 2 KiB for 128 MiB | 512 |

One reserved sector, two FATs sized to cover the data area they describe, media byte `0xF8`, and the `"FAT12"` or `"FAT16"` type string at offset 54 that `Filesystem::new` looks for; a volume of more than 65535 sectors keeps its size in the 32-bit field. It refuses a device too small for any data or too large for FAT16 (about 2 GiB), and the narrow band of sizes just past FAT12 that FAT16 at 512 B clusters cannot take.

Only the boot sector, the FATs and the root directory are cleared. The data area is left as it is: no cluster in it is reachable until the FAT hands it out, and whatever takes one clears what it needs to. Clearing it all took as long as writing every sector of the disk, a long pause at every boot on a RAM disk of a sixteenth of the memory.
