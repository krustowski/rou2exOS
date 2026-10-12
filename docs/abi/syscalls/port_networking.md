# Port I/O and Networking

## 0x47 (RNTP network disk bridge)

Attach/mount a remote FAT12/16 volume, poll and complete sector requests,
detach/unmount, or format a server RAM image. See the complete
[RNTP bridge ABI](../../networking/rntp.md#kernel-bridge-syscall-0x47),
structures and ownership rules. Argument 3 (`RCX`) must be set explicitly.

## 0x30 (Send value to port)

Write one byte to a hardware I/O port. Both args are pointers — the kernel dereferences them.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to port number (`*const u16`) | pointer to value (`*const u32`; low byte written) | ✅ |

## 0x31 (Receive value from port)

Read a 32-bit value from a hardware I/O port. Both args are pointers — the kernel dereferences them.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to port number (`*const u16`) | pointer to output (`*mut u32`; receives the result) | ✅ |

## 0x32 (Serial port)

UART port COM1.

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|-----|
| `0x01` | `0x00` | Serial port initialization. | ✅ |
| `0x02` | pointer to value | Read from the serial port. | ✅ |
| `0x03` | pointer to value | Write to the serial port. | ✅ |

## 0x33 (Create packet)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|-----|
| `0x01` | pointer to buffer | Create an IPv4 packet. | ✅ |
| `0x02` | pointer to buffer | Create an ICMP packet. | ✅ |
| `0x03` | pointer to buffer | Create a TCP packet. | ✅ |

IPv4 requests include the header in `total_length`. A complete 1,500-byte
packet fits; the builder validates the header and total lengths before copying.

TCP requests use the packed 30-byte `TcpPacketRequest` (`TcpHeader`, source
IPv4, destination IPv4, native `length: u16`), followed by the declared body.
Header fields are native endian. The returned segment is in network order,
including its checksum and the requested receive window. Data-offset values
zero (legacy) and five select a plain 20-byte header; six through fifteen
select TCP options at the start of the body, followed by application data.
`length` includes those options. The body limit is 1,460 bytes. Invalid data
offsets, truncated options or oversized requests return `InvalidInput`.
Allocate enough space for the request and output, with a minimum of 512 bytes
for the existing output-clearing behavior.

## 0x34 (Send frame/packet)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01` | pointer to buffer | Send an IPv4 packet over the legacy serial/SLIP path (derives packet length from the IP header). | ✅ |
| `0x04` | pointer to raw Ethernet frame | Send a raw Ethernet frame. Length is derived from the EtherType field (`0x0800` = IPv4, `0x0806` = ARP). A frame for this machine itself (`127.x`, its own address) goes through the loopback device instead of the NIC; see [Networking](../../networking/overview.md#loopback-device). | ✅ |

For raw Ethernet IPv4 transmission, a frame from the registered tunnel source address to a configured prefix is queued to the tunnel daemon before loopback/NIC transmission. A full daemon queue returns `Busy`, without sending the frame in plaintext. The daemon's own outer UDP bypasses this rule. See [Userspace IPv4 tunnel](#0x44-userspace-ipv4-tunnel).

## 0x35 (Socket receive)

Pops a message from the calling process' MQ. Non-blocking returns `0` immediately if the queue is empty; blocking suspends the process until a frame arrives.

An empty blocking call can return `0` after waking; retry to consume the queued
frame. The kernel is not given a destination capacity and copies the message's
frame length (or 512 bytes for a legacy message); reserve 2 KiB for Ethernet.
See [Receiving](../../multitasking/IPC.md#receiving-syscall-0x35-receive_data).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x00` = non-blocking, non-zero = blocking | pointer to buffer | ✅  |

## 0x36 (Socket send)

The legacy implementation copies 512 bytes to `MSG_BUF[0]`, attempts to queue
a message and wakes the target. Argument 1 currently indexes a scheduler slot,
rather than resolving a public PID. It also enqueues `buf_addr = 512` instead
of the payload address and does not report queue failure. This path does not
provide reliable payload delivery; see [IPC limitations](../../multitasking/IPC.md#legacy-send-path).

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| target scheduler slot | pointer to 512-byte buffer | ✅ (legacy limitations above) |

## 0x37 (Register Ethernet driver, bind TCP or UDP port)

Ethernet driver registration, or port binding. Both are released when the calling process exits, is killed or crashes.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0` for global driver | ignored | ✅ |
| TCP port number (`1`–`65535`) | `0` to bind, `1` to unbind | ✅ |
| UDP port number (`1`–`65535`) | `2` to bind, `3` to unbind | ✅ |

TCP and UDP have independent tables of 16 entries each. The same numeric
port can belong to different processes for the two protocols. UDP routing
accepts complete, unfragmented IPv4 datagrams with valid header lengths;
protocol parsing and checksum verification belong to the receiving stack.
ARP replies are copied once to each process with either kind of binding.
The existing `0x38` port list and `0x46` owner table continue to list TCP only.

Binding a port already owned by the caller is idempotent. Binding another
process's port or binding when all 16 entries are occupied returns `Busy`
(`0xfa`), leaving existing bindings intact. Unbinding an absent port succeeds;
unbinding another process's port returns `Busy`. Unknown operations return
`InvalidInput`. Global registration returns `NotImplemented` when no supported
NIC is found. See [Driver and Port Registry](../../networking/overview.md#driver-and-port-registry).

## 0x38 (Get networking status)

Query network status (read-only). Writes `{ mac[6], ip[4], drv_active, n_ports, ports[16] }` into the struct.

Returns `InvalidInput` on invalid pointer, `Ok` otherwise.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to `NetStatus` struct | *unused* | ✅ |

## 0x3d (Network configuration)

The machine's network configuration: IPv4 address, netmask, gateway, the
gateway's MAC, DNS and the card's MAC, as the global Ethernet driver (`eth`)
got them by DHCP or was given them. Every network stack can read it instead of
assuming addresses of its own. The published gateway MAC avoids repeating ARP
discovery; [ARP replies](../../networking/overview.md#frame-routing) also reach
processes with bound TCP or UDP ports.

With Argument 1 `0x01` the kernel fills the [`NetConfig`](../type_definitions.md#netconfig-syscall-0x3d) at Argument 2; the IP and MAC come from the system configuration (the same ones `0x01` and `0x38` report), the rest is what the driver last set. Unset fields are zero.

With Argument 1 `0x02` the calling process sets it from the struct at Argument 2. Only the registered global driver (`0x37` with port 0) may; anyone else gets `InvalidInput`. The IP is stored as `0x01`/`0x02` would store it; the MAC is the card's and is ignored. `Busy` means the system configuration was locked that instant: try again.

Returns `InvalidInput` on an invalid pointer or Argument 1, `Ok` otherwise.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x01` read, `0x02` set | pointer to `NetConfig` struct | ✅ |

## 0x44 (Userspace IPv4 tunnel)

Register one userland tunnel daemon, deliver its outer UDP and outgoing inner frames, and inject frames that the daemon has authenticated. The kernel stores routing metadata and process ownership; WireGuard keys and protocol state remain in [wgd](../../networking/wireguard.md).

Argument 1 (`RDI`) is the operation, Argument 2 (`RSI`) is a pointer or zero, and Argument 3 (`RCX`) is the frame length or zero. **Set `RCX` explicitly on every call**, including the capability query. The syscall number is `0x44` in `RAX`.

| Operation | Argument 2 | Argument 3 | Result |
|-----------|------------|------------|--------|
| `0` — query ABI | `0` | `0` | Capability `0x52325748` (ABI 2) |
| `1` — attach daemon | Pointer to 140-byte `WgdRegistration` | `0` | `Okay` or `Busy` |
| `2` — detach | `0` | `0` | `Okay` or `Busy` if another process owns the tunnel |
| `3` — inject authenticated frame | Pointer to complete Ethernet/IPv4 frame | Frame length, 34–1434 bytes | `Okay` or `Busy` |
| `4` — claim ICMP probe | Pointer to 12-byte input/output `WgdProbe` | `0` | `Okay` or `Busy` |
| `5` — release ICMP probe | `0` | `0` | `Okay` or `Busy` if the caller holds no probe |

Unknown operations, invalid buffers or invalid argument combinations return `InvalidInput`. Operations 1 and 3 return `Busy` for rejected registration/frame contents as well as ownership or queue failures; callers must not assume it always means retrying will succeed. Older kernels can lack the syscall or report the incompatible ABI 1 capability `0x52325747`.

### Structures

These C layouts match the kernel's `#[repr(C)]` structures. IPv4 bytes are in network order; the `uint16_t` fields use native x86 little-endian order. Zero-initialize structures, including reserved bytes.

```c
#include <stdint.h>

typedef struct {
    uint8_t network[4];
    uint8_t prefix;
    uint8_t reserved[3];
} WgdRoute;                         /* 8 bytes */

typedef struct {
    uint8_t local[4];
    uint16_t port;
    uint16_t mtu;
    uint8_t count;
    uint8_t reserved[3];
    WgdRoute routes[16];
} WgdRegistration;                  /* 140 bytes */

typedef struct {
    uint8_t target[4];
    uint8_t local[4];
    uint16_t id;
    uint16_t reserved;
} WgdProbe;                         /* 12 bytes */
```

`WgdRegistration` offsets are `local` 0, `port` 4, `mtu` 6, `count` 8, `reserved` 9 and `routes` 12. `WgdProbe` offsets are `target` 0, `local` 4, `id` 8 and `reserved` 10. The shared app header is `c/wgd/tunnel_abi.h`, using the names `wgd_route`, `wgd_registration` and `wgd_probe_registration`.

### Daemon ownership and routing

Attach requires a nonzero UDP port, MTU 576–1420, 1–16 active routes, and a local IPv4 address whose first octet is neither 0 nor 127 and is below 224. Each active route must have a prefix length 0–32, a normalized network address and zero reserved bytes. Registration reserved bytes must also be zero. Only one process can own the tunnel; the owner may replace its own registration, which also clears any probe reservation.

Incoming outer IPv4 UDP addressed to the registered port is queued to the daemon. In syscall `0x34`, raw Ethernet IPv4 frames from another process with `source == local` and a destination matching a registered prefix go to the daemon before loopback/NIC transmission. A full queue returns `Busy`; the selected frame is never sent through the NIC as a fallback. The daemon's own outer frames bypass this routing rule. Physical NIC input whose source or destination equals the tunnel's local address is rejected.

Injection is accepted only from the owner. The frame must contain a complete, bounded, unfragmented IPv4 packet within the configured MTU, with protocol TCP or ICMP, destination equal to `local`, and source in a registered prefix but different from `local`. TCP is delivered through the usual port/driver demultiplexer. ICMP is delivered to a matching probe owner; accepted ICMP without a matching probe is discarded with `Okay`. A full delivery queue returns `Busy`.

Detaching releases the tunnel and probe state. Process exit, kill or crash releases owned registrations and queued frames; a probe client's exit releases its reservation without stopping the daemon.

### ICMP probes

For operation 4, fill `target` with the desired IPv4 address; the other input fields are ignored. On success the kernel overwrites the structure with the target, local tunnel source address, an ICMP identifier and zero reserved field. The shell uses that address and identifier for echo requests sent through syscall `0x34`.

The target must match a registered prefix, differ from the local address, and have a first octet neither 0 nor 127 and below 224. A tunnel must be attached and the caller must differ from its owner. Only one process can reserve a probe at a time; the same process may replace its reservation. Failure returns `Busy`.

Authenticated echo replies match the target and identifier. ICMP time-exceeded and destination-unreachable replies match the quoted original IPv4/ICMP request's source, target and identifier. The shell additionally verifies checksums, sequence and echo payload. Reply source addresses, including intermediate routers, must pass the tunnel's normal source-prefix check. See [Ping and traceroute](../../networking/wireguard.md#ping-and-traceroute-through-subnets) for user-facing commands.


## 0x45 (Network throughput statistics)

Read-only, coherent [NetStats](../type_definitions.md#netstats-syscall-0x45)
snapshot of the physical NIC's traffic since boot, on
RTL8139, E1000 and BCM57765. Argument 1 points to a **40-byte** `NetStats`; arguments 2
and 3 are unused (SDK wrappers pass zero). No driver/card is required: totals
are zero before traffic starts and survive driver restarts/NIC reinitialization.

```c
typedef struct {
    uint64_t timestamp_ms;
    uint64_t rx_frames;
    uint64_t rx_bytes;
    uint64_t tx_frames;
    uint64_t tx_bytes;
} NetStats;                         /* 40 bytes */
```

Fields are native little-endian u64 values at offsets 0, 8, 16, 24 and 32.
`timestamp_ms` uses the same millisecond clock as syscall `0x04`. A consumer
computes bytes/second as `(new_bytes - old_bytes) * 1000 / (new_ms - old_ms)`
when time advanced, and starts a fresh baseline if counters decreased.

RX counts each valid frame when consumed from the NIC ring, including frames
discarded by routing or by an unresponsive receiver. Peeking/retrying a held
frame and copying an ARP reply to several processes do not multiply the count.
Malformed descriptors discarded internally by the card driver and frames lost
before software reads them are excluded. TX counts frames successfully submitted
to the card, not completion or peer delivery; failed submissions are excluded.
Bytes include the Ethernet header and padding (at least 60 bytes for TX), but
exclude the FCS, preamble and inter-frame gap. RTL8139's attached RX FCS is
subtracted; E1000 strips it in hardware. Loopback, serial SLIP and tunnel inner
frames are excluded; physical tunnel outer frames count normally. Totals wrap
modulo 2^64 and reads never reset them.

Returns `Okay` (`0`) or `InvalidInput` (`0xfc`) if the entire output buffer
is not in a user region. The kernel writes nothing on error. An older kernel
returns `InvalidSyscall` (`0xff`).

## 0x46 (TCP port owners)

Read-only atomic [NetPortTable](../type_definitions.md#netporttable-syscall-0x46)
snapshot of the TCP destination port registry and process
identities. Argument 1 points to a **528-byte** `NetPortTable`; arguments 2
and 3 are unused. This includes client reply ports as well as service ports.

```c
typedef struct {
    uint16_t port;
    uint8_t reserved[6];
    uint64_t pid;
    uint8_t name[16];
} NetPortBinding;                   /* 32 bytes */

typedef struct {
    uint64_t driver_pid;
    uint8_t n_ports;
    uint8_t reserved[7];
    NetPortBinding bindings[16];
} NetPortTable;                     /* 528 bytes */
```

The header has `driver_pid` at offset 0, `n_ports` at 8 and `bindings` at 16.
Each entry has `port` at offset 0, `pid` at 8 and `name` at 16. Integer fields
are native little-endian. `driver_pid == UINT64_MAX` means no global driver.
Only `bindings[0..n_ports]` are live (at most 16); unused entries and reserved
bytes are zero. Names contain up to 16 bytes and need not have a trailing NUL.

The internal routing registry stores reusable scheduler slots. This call
resolves them under the scheduler lock with interrupts disabled to expose full
64-bit `Process::id` values and names, rather than slots or the truncated u8
IDs in legacy `0x2f`. Binding release and exit/kill/crash cleanup are reflected
in the next snapshot. Readers may scan the table for one port.

Returns `Okay` (`0`), `Busy` (`0xfa`) if the scheduler lock is held (retry),
or `InvalidInput` (`0xfc`) for an invalid output buffer. Errors leave the
buffer untouched; an older kernel returns `InvalidSyscall` (`0xff`).

SDKs provide `get_net_stats`, `get_net_ports`, `net_port_owner` (C);
`r2::net::stats`, `ports`, `port_owner` (C++); and `ReadNetStats`,
`ReadNetPorts`, `NetPortOwner` (Go).

---

Related documentation: [ABI Specification](../syscall_specification.md), [Type Definitions](../type_definitions.md), [Networking Overview](../../networking/overview.md), [IPC](../../multitasking/IPC.md).
