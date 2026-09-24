# FAT12

FAT12 is the primary read/write filesystem, stored on a 1.44 MB floppy disk. It is accessible as:

- Absolute paths under `/mnt/fat/`
- Bare filenames relative to the current working directory (cluster stored in `SYSTEM_CONFIG`)

---

## Hardware: Floppy Controller (`fs/fat12/block.rs`)

`Floppy` implements `BlockDevice` using direct ISA DMA and FDC I/O port programming.

### DMA Setup (`Floppy::init`)

Called before every read. Programs ISA DMA channel 2 to transfer one sector (512 bytes) from the floppy controller into the static `DMA: [u8; 512]` buffer:

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
4. Wait for IRQ6 by polling `port 0x3F4` MSR bit 7, then send Sense Interrupt (0x08) and drain 7 result bytes.
5. `copy_nonoverlapping(DMA, buffer, 512)`.

The floppy disk geometry assumed throughout: 80 cylinders, 2 heads, 18 sectors/track = 1440 sectors × 512 bytes = 1.44 MB.

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
| 19 | `total_sectors_16` | Total sectors on disk |
| 22 | `fat_size_16` | Sectors per FAT copy (usually 9) |

Detection: `Filesystem::new` scans the boot sector for the 5-byte string `"FAT12"`. If not found, returns `Err`.

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

`FatTable` reads all 9 FAT sectors (4608 bytes) into a single `[u8; 4608]` for batch inspection. `Filesystem::read_fat12_entry` reads individual sectors on demand, handling the cross-sector boundary case (when `fat_offset == 511`).

`write_fat12_entry` updates **every** FAT copy (`fat_count`), not just the first, so a checker comparing them does not report the volume as damaged. `allocate_cluster` scans the FAT one sector at a time (one read per FAT sector, not per cluster) and returns `0` when the FAT is full.

---

## Operations

### Read File (`read_file`)

Follows the cluster chain starting from `start_cluster`:

1. Convert cluster → LBA.
2. Read 512-byte sector into the caller's buffer at offset `count × 512`.
3. Advance to `read_fat12_entry(cluster)`.
4. Stop when chain entry ≥ `0xFF8` or buffer is exhausted.

### Write File (`write_file`)

1. If a file with the same name exists: `free_cluster_chain` its old clusters, mark its directory entry `0xE5`.
2. `allocate_cluster()`: scan FAT entries for `0x000` (free), mark it `0xFFF`, return its index.
3. Write each 512-byte sector of `data` into allocated clusters; `write_fat12_entry(cluster, next)` to chain them; mark the last cluster `0xFFF`.
4. `write_dir_entry()`: scan the directory for a free slot (`0x00` or `0xE5`), write the 32-byte entry (name, attr `0x20`, cluster, file_size).

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
3. Zero the cluster's sector.
4. Write `.` (self-pointer) and `..` (parent-pointer) entries into the new cluster.
5. Mark the new cluster as end-of-chain (`0xFFF`) in the FAT.

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
