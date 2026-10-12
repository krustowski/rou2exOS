# RNTP network disk

RNTP (Rourex Network Talk Protocol) v1 exports a RAM volume from one R2
instance to multiple R2 or Linux clients over TCP. The server formats a fresh export using R2's
FAT formatter: 2 MiB uses FAT12, larger supported sizes use FAT16. The client
mounts the volume at `/mnt/net` after validating the handshake and boot sector.
The usual file, directory, range-I/O and filesystem-size syscalls work there.
Memento's Files window recognizes mount type `6` as a writable netdisk; the
C, C++ and Go SDKs expose the same mount constant.

## Build and connect

`make build_rntp` builds the service in the sibling `r2_app/c/rntp` directory
against `r2_app/c/libcr2.a` and stages `iso/bin/rntp.elf`. `make build_iso` also builds/stages it. Rebuild
both kernels and the boot archive/ISO to include syscall `0x47` and the service.
The executable is available from `/mnt/tar/bin` and `/mnt/iso/bin`.

To build or test it directly, run `make -C ../r2_app/c/rntp build` or
`make -C ../r2_app/c/rntp test`. The app owns both server/client code and the
portable protocol tests; the kernel owns the bridge and guest integration tests.
Kernel build targets accept `R2_APP=/path/to/r2_app` for another checkout.

On the server, after ETH has configured its address:

```text
bg rntp serve 16
```

On the client, using the server's IPv4 address:

```text
bg rntp connect 192.0.2.1
mount
cd /mnt/net
write HELLO hello-from-r2
read HELLO.TXT
```

The default TCP port is 7780. A custom port is the last argument:
`rntp serve 2 7781` and `rntp connect 192.0.2.1 7781`. Export sizes are whole
MiB, from 2 through 64, subject to available user heap memory. The client chooses
a separate local TCP port. Put either command in INIT.RC after `bg eth` when
the peer address and role are known. A connection failure prints an error;
there is no automatic reconnect loop.

For two guests on an isolated Ethernet link, start ETH with distinct addresses
first: `bg eth --ip 192.0.2.1 --mask 255.255.255.0` on the server and
`bg eth --ip 192.0.2.2 --mask 255.255.255.0` on the client.

## Linux FUSE mount

Install a C compiler, `pkg-config`, libfuse3's development package and
`fusermount3`. Fedora names the development package `fuse3-devel`;
Debian/Ubuntu use `libfuse3-dev`. Build and mount from `r2_app/c/rntp`:

```sh
make linux # or: make build_rntp_linux from r2_main
mkdir -p ~/rntp
./rntp-fuse 192.0.2.1 ~/rntp -f
# Another shell:
cp picture.bmp ~/rntp/PICTURE.BMP
ls ~/rntp
fusermount3 -u ~/rntp
```

Use `-p 7781` for another TCP port. Omit `-f` to run in the background. FUSE
options follow the mountpoint; `-o allow_other` uses the host's normal FUSE
permission configuration. Each Linux instance and R2 instance sees the same
files, directories, renames and deletions. The mount connects to the server;
it does not format the existing export.

The Linux client implements FAT12/16 in userspace and participates in RNTP's
operation locks. It supports reads/writes, atomic append per FUSE request,
create, mkdir, unlink/rmdir, rename (including replacement and NOREPLACE),
truncate, DOS modification timestamps and statfs. READ_BATCH prefetches up to
32 sectors; writes coalesce into 32-sector batches. Metadata and data use
separate bounded caches, both invalidated after a foreign revision change.
Kernel page/attribute/name caches and writeback are disabled to observe peer
changes; each write waits for commit. `statfs` uses committed server STAT data.

Names are case-insensitive FAT 8.3 names; Linux rejects longer names rather
than truncating them. This mount has FAT's fixed permissions and UTC DOS times
with two-second precision; symlinks, Unix ownership/mode changes, long names,
distributed advisory locks and mmap are unsupported. Deleted or remotely
renamed open files do not retain POSIX open-file lifetime semantics. Use
separate files or application coordination for longer editing sessions:
RNTP locks individual filesystem operations, not an application's entire edit.
`fsync` acknowledges the RAM export and cannot make it survive a server reboot.

## Lifetime and failures

The server supports up to seven connected clients sharing the same filesystem.
It never mounts its export locally. Updated clients use shared sessions and
hold a server lock for each FAT operation, including reads. Each commit changes
the volume revision when sectors were written; the next client invalidates its
metadata cache before accessing that revision. A second daemon on the same R2
still cannot claim its one `/mnt/net` mount.

Legacy HELLO clients retain an exclusive lease and cannot coexist with shared
sessions. Updated R2 clients fall back to an exclusive connection when either
peer's kernel/service lacks shared support. Linux requires the shared extension.
The server allocates an undo image equal to the export size, plus a sector
bitmap, so a 64 MiB export needs roughly 128 MiB for its image and undo storage.

The server's allocated image survives client disconnects; a reconnect mounts
its existing FAT volume without formatting it. Stop the client with `kill` using the task ID from `ts`; the
kernel releases its registration and removes `/mnt/net`. Reconnect by starting
`rntp connect` again. The server releases a vanished client's lease within ten
seconds; it may report `BUSY` until then.

Data is volatile: stopping/restarting the server or rebooting it loses the
export. A write response means the sector has reached the server's RAM. RNTP
v1 has no durable backing or durable flush operation. It trusts peers on its
network and does not authenticate or encrypt connections.

The client sends heartbeats every two seconds. A failed exchange ends after
five seconds and unmounts the disk; the kernel has a ten-second block-I/O
deadline even if the client daemon stops servicing requests. Reads and writes
report the calling syscall's normal error value (`FilesystemError`, or
`u64::MAX` for range I/O/directory listing). A failed FAT sector stops further
writes in that filesystem operation. Shared sessions keep an undo copy of each
changed sector until UNLOCK commits the operation. ABORT, socket reset or lease
expiry restores all those sectors before another client can acquire the lock.
Killing an R2 process inside an operation disconnects its daemon registration
so the server can roll back its writes. A lost commit reply can still mean that
the operation committed; clients do not automatically replay writes. Legacy
exclusive clients retain their previous unjournaled sector-write behavior.

Mount sizes and free space (syscall `0x40`, the shell's `mount` and Memento's
Files window) read a connection-scoped kernel cache. They never wait for TCP
or acquire the netdisk's filesystem operation lock. The server counts free
clusters directly in its RAM image. The client fetches the initial snapshot
before mounting and refreshes it every two seconds using STAT as its heartbeat,
including during transfers. Free space can briefly lag a write or deletion;
disconnect and a new attachment clear the previous cache. Both peers need the
updated RNTP service for the STAT extension. Legacy attach without a published
snapshot reports unavailable usage immediately rather than scanning the peer.

Whole FAT operations are serialized while their network requests run. The
kernel keeps interrupts and scheduling active during waits. Connection epochs
and request IDs prevent late completions from satisfying a later request;
attach is refused until an old filesystem operation has unwound. Process death
cancels an outstanding request and releases operation ownership.

The client caches up to 192 boot, primary FAT and root-directory sectors
(96 KiB of sector data). File data bypasses this cache, so a bulk transfer
cannot evict the FAT. Reads reuse acknowledged metadata. Disconnects,
failed responses and interrupted filesystem operations invalidate it,
including reconnects to the same server generation. Shared clients also
invalidate FAT/root metadata whenever LOCK returns a different volume revision.
The shared boot sector is immutable, so its validated geometry remains cached.

When CAPS advertises READ_BATCH, R2 downloads fetch up to 32 consecutive sectors
(16 KiB) per TCP exchange. Bridge operation 12 publishes the acknowledged batch
into a separate kernel read buffer; the remaining sector reads in that FAT
operation avoid daemon wakeups and network round trips. The buffer is discarded
on operation end, writes, failures, disconnects and foreign revision changes.
Metadata-only reads stop before the data area. The daemon uses its own bounded
read buffer on older kernels; servers without READ_BATCH retain sector reads.
Image loading and other range reads use this path without changing syscall
`0x39` or the fmimage job ABI. Update the client kernel and both RNTP apps for
the full optimization.

After CAPS negotiation, the kernel stages up to 32 distinct sector writes
within the current filesystem operation. Repeated FAT edits and a cluster's
zeroing followed by its data write coalesce into their final sector contents.
Reads by that operation see its staged contents. The write set flushes when
full, before an uncached remote read, and before the outermost operation ends.
Every filesystem syscall waits for these acknowledgments before returning;
there is no background write-behind. Failed flushes return the syscall's
normal error and unmount the export. No unacknowledged sectors enter the
shared metadata cache or survive a disconnected/interrupted operation.

Memento copies and moves files in a separate `fmcopy.elf` process, built with
the desktop and staged in `iso/bin` by `make build_iso`. The worker performs
replacement, range I/O, source deletion for moves, and the final directory
listings. The Files window only polls progress and paints cached listings.
Escape requests cancellation between chunks; closing the window also cancels
the worker and retains its shared state until it stops. A cancelled or failed
copy can leave a partial destination; the source is removed only after a
complete successful move. The desktop stops outstanding workers on shutdown.

The Image window also loads in the background using `fmimage.elf`, staged
alongside Memento. It checks Jug's `/mnt/tmp/jug/FMIMAGE.ELF` first, then
`/mnt/tar/bin/fmimage.elf`, `/mnt/iso/bin/fmimage.elf`, and finally a
floppy-root copy at `/mnt/fat/FMIMAGE.ELF`. Explicit local paths keep a
netdisk working directory from delaying the launch. File-size
lookup, bounded reads and PNG/JPEG/GIF/BMP
decoding run in that process. The window shows loading progress, continues
handling input during a slow or stalled netdisk read, and closes with Escape
or Back. Its shared job stays alive until a cancelled loader has stopped.
Decoded pixels belong to the desktop and remain valid after the loader exits;
desktop shutdown stops pending loaders before releasing their shared memory.
Loading progress repaints at most every 200 ms, while heartbeat/input polling
continues; phase changes and completion repaint immediately.

## Wire protocol

Each message has a 40-byte header followed by its declared payload. All
multibyte wire fields are big endian; no host pointers appear on the wire.

| Offset | Bytes | Meaning |
|--------|-------|---------|
| 0 | 4 | ASCII `RNTP` |
| 4 | 1 | Version, `1` |
| 5 | 1 | Opcode; responses set bit `0x80` |
| 6 | 2 | Status: `0` okay, `1` busy, `2` invalid, `3` stale generation |
| 8 | 4 | Payload length: Declared payload size, at most 16,644 bytes |
| 12 | 4 | Reserved, zero |
| 16 | 8 | Nonzero request ID, echoed in the response |
| 24 | 8 | Sector LBA; HELLO response carries export capacity in sectors |
| 32 | 8 | Export generation; zero in initial HELLO |

Opcodes are `1 HELLO`, `2 READ`, `3 WRITE`, `4 PING`, `5 STAT`, `6 CAPS`,
`7 WRITE_BATCH`, `8 HELLO_SHARED`, `9 LOCK`, `10 UNLOCK`, `11 READ_BATCH` and
`12 ABORT`. HELLO and HELLO_SHARED use zero generation/LBA and no payload;
replies contain sector zero, export capacity and its connection generation.
HELLO claims an exclusive export; HELLO_SHARED joins the shared filesystem.
Every later request carries that generation. READ returns one sector; WRITE
carries one sector and returns no payload. PING has no payload. Shared READ,
WRITE and batch operations require the requesting session to hold LOCK.

CAPS uses LBA zero and no payload; its reply is an eight-byte capability mask:
`0x01` WRITE_BATCH, `0x02` shared sessions and `0x04` READ_BATCH. WRITE_BATCH
uses LBA zero and `count: u32`, then 1–32 records of `lba: u64, data[512]`.
The maximum payload is 16,644 bytes. The server validates all records before
changing any sector. Its empty reply acknowledges the complete batch, whose
writes remain undoable until UNLOCK. Shared writes to sector zero are refused.
READ_BATCH carries a four-byte count (1–32), starts at the header's LBA and
returns `count * 512` consecutive bytes. Both Linux and R2
clients use it for bounded read-ahead.

LOCK, UNLOCK and ABORT use LBA zero and no payload. Successful replies carry
an eight-byte, nonzero volume revision. LOCK returns BUSY while another
operation holds it; that response keeps the connection open and clients retry
within their bounded deadline. UNLOCK commits staged changes and increments
the revision only if sectors were written. ABORT restores the undo sectors
without changing the revision. Disconnect and lease expiry also abort. A
revision describes committed disk contents; it differs from the export
generation, which stays unchanged across commits and client reconnects.

STAT uses LBA zero and no payload; its eight-byte reply is the free-space
count in bytes from the last committed image. The server counts clusters in
RAM when committing changes. Heartbeats and other clients never see free
space from an unfinished transaction.

The service preserves partial headers, split payloads and coalesced messages.
It sends multiple segments within the peer's receive window and negotiates
an MSS of 1,460 bytes for the 1,500-byte Ethernet MTU. RNTP provides a 32 KiB
TCP receive buffer without changing libcr2's public socket layout. The kernel
preserves its actual remaining window in each wire header; it previously
overrode the application with a constant 1,024 bytes. TCP construction carries
SYN options through IPv4/Ethernet and accepts full MTU packets. Other apps
retain plain SYNs, their existing buffers and a 1,370-byte sending limit unless
they opt in with `tcp_set_mss()`. Draining a full socket buffer sends a window
update, so a peer can resume without waiting for another application write.
RNTP also enables `tcp_set_local_packet_crafting()` to build IPv4/TCP headers
and checksums in userspace, removing two packet-builder syscalls per segment.
Raw Ethernet transmission still goes through the kernel. This works on older
netdisk kernels; applications using kernel packet construction rebuild a
plain SYN and retain the old sending limit if that kernel ignores SYN options.
Bursts are paced at eight segments per daemon
turn. Cumulative acknowledgments advance the send window, and unacknowledged
bytes retransmit at their original TCP sequence numbers after 250 ms, so lost
segments cannot resubmit an application write. Application requests remain
serialized; write batching amortizes that round trip across many sectors.
Kernel block waits sleep until the next timer tick, allowing the transport
daemon and UI to run without repeatedly polling all devices in a busy loop.

## Kernel bridge: syscall `0x47`

Argument 1 is the operation, Argument 2 a buffer or zero, Argument 3 (`RCX`)
the format buffer length or zero. Set `RCX` explicitly for every call. The
native x86 C layouts are defined in `r2_app/c/rntp/bridge.h`.

| Operation | Buffer | Result |
|-----------|--------|--------|
| 0 | Zero, length zero | ABI tag `0x52324e44` |
| 1 | `NetDiskAttach`, 528 bytes | Attach and mount |
| 2 | `NetDiskRequest`, 536 bytes | `1` request copied, `0` no request |
| 3 | `NetDiskRequest`, 536 bytes | Complete the matching request |
| 4 | Zero, length zero | Detach/unmount; owner only while attached |
| 5 | Shared heap image, length 512–64 MiB, a multiple of 512 | Format a new FAT12/16 image |
| 6 | `NetDiskUsageUpdate`, 16 bytes | Publish cached free space; owner/generation must match |
| 7 | `NetDiskAttachInfo`, 536 bytes | Attach and mount with an initial usage snapshot |
| 8 | `NetDiskAttachInfo`, 536 bytes | Attach with usage and operation-scoped write buffering |
| 9 | `NetDiskBatch`, 16,656 bytes, aligned to 8 | Poll a write batch: `1` copied, `0` none |
| 10 | `NetDiskBatchReply`, 16 bytes | Acknowledge matching batch ID and count |
| 11 | `NetDiskAttachInfo`, 536 bytes | Attach with batching and shared operation locks/revision invalidation |
| 12 | `NetDiskReadAheadReply`, 16,408 bytes, aligned to 8 | Complete a READ and publish 1–32 acknowledged sectors |
| 13 | Zero, length zero | Supported read-ahead sector count, `32` |

`NetDiskAttach` contains `sectors: u64`, `generation: u64`, `boot[512]`.
`NetDiskRequest` contains `id: u64`, `lba: u64`, `op: u32`, `status: u32`,
`data[512]`. Bridge operations return `Okay` (`0`), `Busy` (`0xfa`),
`InvalidInput` (`0xfc`) or `FilesystemError` (`0xfd`), except the query/poll
results above. A nonzero completion status fails the operation. Only the
registered client daemon can poll/complete. Formatting modifies the supplied
image; it does not attach or mount it. The mount type reported by `0x2c` and
`0x40` is `6` (`netdisk`); format is `1` FAT12 or `2` FAT16.

`NetDiskUsageUpdate` contains `generation: u64`, `free_bytes: u64`.
`NetDiskAttachInfo` appends `free_bytes: u64` to `NetDiskAttach`. Geometry and
format come from the validated boot sector; free bytes must be cluster-aligned
and fit within the data area. Operation 7 initializes the cache atomically with
the mount. An operation-1 attach has no known free space until operation 6.

`NetDiskBatch` is `id: u64, count: u32, reserved: u32`, followed by 32 native
`{lba: u64, data[512]}` records; only the first `count` are valid. Reserved
is zero. `NetDiskBatchReply` contains `id: u64, count: u32, status: u32`.
Only the attached daemon can poll/complete. Operation 8 enables batching
atomically with attachment; operations 1 and 7 keep their legacy behavior.
Operation 11 also makes the kernel submit wire opcodes LOCK, UNLOCK and ABORT
through the existing 536-byte request bridge. Successful control completions
put the big-endian revision in the first eight data bytes, without caching it
as a disk sector. Local nesting acquires/releases the server lock only at the
outermost filesystem operation.

`NetDiskReadAheadReply` contains `id: u64, lba: u64, count: u32, status: u32`
and `data[32][512]`; only the first `count` sectors are read. Operation 12
requires a delivered pending READ with matching ID/LBA, valid count and bounds.
The kernel copies directly from the checked daemon buffer into static storage
without placing a 16 KiB value on the kernel stack. Operation 13 is a capability
query; an older kernel returns InvalidInput and the daemon uses sector
completions. Wire READ_BATCH replies must match the requested byte count before
any sectors enter the read buffer.

## Verification

`make test` includes real FAT formatter/filesystem tests and bridge ownership
and failure tests, then runs `r2_app/c/rntp`'s wire codec/server-handler tests.
For actual
R2 guests, after building the service and kernel:

```sh
python3 tests/netdisk/qemu_pair.py --size 2
python3 tests/netdisk/qemu_pair.py --read-benchmark --no-loss --copy-kib 512 --size 16 --require-read-throughput --output /tmp/r2-read-speed
python3 tests/netdisk/qemu_pair.py --size 16 --output /tmp/r2-netdisk-fat16
python3 tests/netdisk/qemu_pair.py --shared --size 2 --output /tmp/r2-netdisk-shared12
python3 tests/netdisk/qemu_pair.py --shared --size 16 --output /tmp/r2-netdisk-shared16
make -C ../r2_app/c/rntp test-linux test-fuse
python3 tests/netdisk/qemu_linux.py --size 2
python3 tests/netdisk/qemu_linux.py --mixed --size 16 --output /tmp/r2-netdisk-linux16
make -C ../r2_app/cpp/memento-hello copytest
python3 tests/netdisk/qemu_pair.py --copy --output /tmp/r2-netdisk-copy
python3 tests/netdisk/qemu_pair.py --desktop --size 16 --output /tmp/r2-netdisk-desktop
python3 tests/netdisk/qemu_pair.py --image-viewer --size 16 --output /tmp/r2-netdisk-image
python3 tests/netdisk/qemu_pair.py --image-viewer --image-loader-jug --size 16 --output /tmp/r2-netdisk-image-jug
python3 tests/netdisk/qemu_pair.py --copy --require-throughput --batch-failure --output /tmp/r2-netdisk-throughput
python3 tests/netdisk/qemu_pair.py --copy-only --no-loss --copy-kib 1024 --require-window --require-mss --output /tmp/r2-netdisk-speed
```

The fixture uses a private loopback Ethernet relay and its own temporary boot
ISO. It checks file bytes across clusters, unaligned range reads, directories,
rename, local mount ownership, client process cleanup, reconnect without format,
lost READ/WRITE TCP segments and a failed read after losing the server. It also
times 100 mount/usage queries with a live and a stalled link, and checks that
background free-space refresh observes file and directory allocations.
`--copy` also checks tar-to-netdisk bytes, worker directory snapshots,
cancellation and polling gaps. `--desktop` boots Memento, starts a copy from
its Files window and checks that UI heartbeats and Escape input continue
during the transfer. `--copy-only` runs the worker test after connecting.
`--image-viewer` opens a 750 KiB BMP from netdisk in actual Memento, verifies
UI heartbeats while loading and that pixels survive the loader's exit, then
stalls the connection and checks that Escape closes the window promptly.
`--image-loader-jug` installs the helper in Jug's download directory and
omits it from the boot `bin` directory to catch hard-coded launch paths.
`--require-throughput` requires at least 32 KiB/s for the copy fixture;
`--rntp-elf` selects another service binary for before/after measurements.
`--kernel` selects the matching kernel. `--copy-kib` accepts 128 (default),
512 or 1024. `--no-loss` is limited to `--copy-only` sustained-throughput
measurements; the regular tests inject loss. `--require-window` and
`--require-mss` inspect real Ethernet frames and require 32 KiB windows and
1,460-byte segments, rather than checking only the userspace packet request.
`--batch-failure` verifies that a disconnected peer fails a buffered write
at its final flush, including the syscall error and unmount. The app's tests
also cover large TCP windows, partial acknowledgments, socket reuse, and
validation of a malformed final batch record without partial application.


`--shared` boots one server and two R2 clients, writes files concurrently and
checks both clients' views after foreign changes. `qemu_linux.py` mounts two
actual Linux FUSE clients against the R2 server using a private QEMU user-mode
network port. `--mixed` also mounts an R2 client and verifies bidirectional
file exchange and foreign revision refresh with Linux. Host `test-linux` covers concurrent TCP clients, out-of-space
rollback and disconnect rollback; `test-fuse` creates and removes two temporary
mounts for both FAT kinds and runs `fsck.fat -n` when available.


`--read-benchmark` writes a known fixture and downloads it in fmimage's 16 KiB
chunks, validating every byte. `--require-read-throughput` requires 128 KiB/s
in this QEMU regression. A 512 KiB FAT16 comparison measured 39 KiB/s with
sector RPCs and 182 KiB/s with batched reads; these are emulator measurements,
not a prediction for physical NICs. The RTL8139 ring boundary regression also
covers a full received frame, including CRC, whose header is at the end of the
ring; its overflow allocation must accommodate the driver's accepted frame
size rather than only an IP packet.
