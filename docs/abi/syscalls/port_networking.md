# Port I/O and Networking

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

## 0x34 (Send frame/packet)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01` | pointer to buffer | Send an IPv4 packet over the legacy serial/SLIP path (derives packet length from the IP header). | ✅ |
| `0x04` | pointer to raw Ethernet frame | Send a raw Ethernet frame. Length is derived from the EtherType field (`0x0800` = IPv4, `0x0806` = ARP). A frame for this machine itself (`127.x`, its own address) goes through the loopback device instead of the NIC; see [Networking](../../networking/overview.md#loopback-device). | ✅ |

For raw Ethernet IPv4 transmission, a frame from the registered tunnel source address to a configured prefix is queued to the tunnel daemon before loopback/NIC transmission. A full daemon queue returns `Busy`, without sending the frame in plaintext. The daemon's own outer UDP bypasses this rule. See [Userspace IPv4 tunnel](#0x44-userspace-ipv4-tunnel).

## 0x35 (Socket receive)

Pops a message from the calling process' MQ. Non-blocking returns `0` immediately if the queue is empty; blocking suspends the process until a frame arrives.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x00` = non-blocking, non-zero = blocking | pointer to buffer | ✅  |

## 0x36 (Socket send)

Copies 512 bytes from the buffer and pushes a message to the target process' MQ, then wakes the target.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| target process PID | pointer to buffer | ✅  |

## 0x37 (Register Ethernet driver, bind TCP port)

Ethernet driver registration, or port binding. Both are released when the calling process exits, is killed or crashes.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| TCP port number (0 for global driver) | unused | ✅ |

## 0x38 (Get networking status)

Query network status (read-only). Writes `{ mac[6], ip[4], drv_active, n_ports, ports[16] }` into the struct.

Returns `InvalidInput` on invalid pointer, `Ok` otherwise.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to `NetStatus` struct | *unused* | ✅ |

## 0x3d (Network configuration)

The machine's network configuration: IPv4 address, netmask, gateway, the gateway's MAC, DNS and the card's MAC, as the global Ethernet driver (`eth`) got them by DHCP or was given them. Every network stack reads it rather than assuming addresses of its own. The gateway's MAC matters most: a process that is not the global driver cannot ARP for it, because the replies go to the driver.

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
