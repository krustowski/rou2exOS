# Protocol Layer

All protocol helpers are **stateless, pure functions** — they take input slices and write into caller-supplied output buffers. No global state, no heap allocation. The only exception is `TcpConnection`, which is a state-machine struct owned by userland.

---

## Ethernet II (`ethernet.rs`)

### Frame Layout

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
├───────────────────────────────────────────────────────────────┤
│                  Destination MAC (6 bytes)                    │
├───────────────────────────────────────────────────────────────┤
│                    Source MAC (6 bytes)                       │
├───────────────────────────────────────────────────────────────┤
│         EtherType (2 bytes)       │      Payload ...          │
└───────────────────────────────────────────────────────────────┘
```

| EtherType | Value |
|-----------|-------|
| IPv4 | `0x0800` |
| ARP | `0x0806` |

### API

- `EthernetFrame::parse(frame)` — borrows a frame slice, returns `dest_mac`, `src_mac`, `ethertype`, and a `payload` sub-slice.
- `EthernetFrame::write(buffer, dest, src, ethertype, payload)` — serialises a frame into `buffer`; returns total byte count.
- `build_ethernet_frame(dst, src, ethertype, payload)` — convenience wrapper, returns a fixed `[u8; 1514]`.

Minimum frame size enforcement (60 bytes) is done at the RTL8139 `send_frame` level, not here.

---

## ARP (`arp.rs`)

ARP is used to resolve an IPv4 address to a MAC address on the local Ethernet segment.

### Packet Layout (28 bytes, Ethernet/IPv4)

```
offset  size  field
  0      2    hw_type    (1 = Ethernet)
  2      2    proto_type (0x0800 = IPv4)
  4      1    hw_len     (6)
  5      1    proto_len  (4)
  6      2    op         (1 = Request, 2 = Reply)
  8      6    sender_mac
 14      4    sender_ip
 18      6    target_mac (zeros for a request)
 24      4    target_ip
```

### API

- `ArpPacket::parse(payload)` — parses an ARP payload (14 bytes after the Ethernet header); returns `sender_mac`, `sender_ip`, `target_mac`, `target_ip`, `op`.
- `ArpPacket::build(buf, op, sender_mac, sender_ip, target_mac, target_ip)` — writes 28 bytes into `buf`.

---

## IPv4 (`ipv4.rs`)

### Header Layout (20 bytes, no options)

```
offset  size  field
  0      1    version (4) | IHL (5)
  1      1    DSCP/ECN (0)
  2      2    total_length  (big-endian)
  4      2    identification (0x1337)
  6      2    flags | fragment_offset  (DF=1, offset=0)
  8      1    TTL (64)
  9      1    protocol (ICMP=1, TCP=6, UDP=17)
 10      2    header_checksum
 12      4    source_ip
 16      4    dest_ip
```

Checksum is the one's complement of the one's complement sum of all 16-bit words in the header (standard RFC 791 algorithm). Set to 0 before calculation.

### API

- `create_packet(src, dst, protocol, payload, out)` — writes header + payload into `out`; fills checksum; returns total byte count.
- `parse_packet(packet)` — returns a copy of `Ipv4Header` and a sub-slice pointing at the payload (past the IHL-derived header end).
- `send_packet(packet)` — SLIP-encodes and writes to UART. **This is the legacy serial path**, not the RTL8139 path.

---

## ICMP (`icmp.rs`)

Implements Echo Request/Reply (type 8 / type 0).

### Header Layout (8 bytes)

```
offset  size  field
  0      1    type  (8 = Echo Request, 0 = Echo Reply)
  1      1    code  (0)
  2      2    checksum
  4      2    identifier
  6      2    sequence_number
```

Checksum covers the full ICMP message (header + payload) with the checksum field treated as 0.

### API

- `create_packet(type, id, seq, payload, out)` — writes ICMP header + payload, fills checksum; returns byte count.
- `parse_packet(packet)` — returns a copy of `IcmpHeader` and a payload sub-slice.

---

## TCP (`tcp.rs`)

### Header Layout (20 bytes, no options)

```
offset  size  field
  0      2    source_port
  2      2    dest_port
  4      4    seq_num
  8      4    ack_num
 12      2    data_offset (top 4 bits = 5) | reserved | flags
 14      2    window_size
 16      2    checksum
 18      2    urgent_pointer
```

Flag constants:

| Name | Bit |
|------|-----|
| `FIN` | 0x01 |
| `SYN` | 0x02 |
| `PSH` | 0x08 |
| `ACK` | 0x10 |

Checksum uses the IPv4 pseudo-header (src IP, dst IP, zero, protocol=6, TCP length) prepended before the one's complement sum. The checksum field at offset 16 is skipped during calculation.

### Connection State Machine

`TcpConnection` holds the per-connection state needed for a simple server:

```
Closed → Listen → SynReceived → Established → FinWait1/FinWait2/Closing/TimeWait
                                             → CloseWait → LastAck
```

State transitions are driven by userland code — the kernel does not run a TCP state machine. The kernel only provides packet construction and checksum helpers.

### API

- `create_packet(src_port, dst_port, seq, ack, flags, window, payload, src_ip, dst_ip, out)` — writes TCP header + payload, fills pseudo-header checksum.
- `parse_packet(packet)` — returns `TcpHeader` and payload sub-slice (respects variable data_offset).
- `parse_flags(header)` — returns `(syn, ack, fin, rst)` booleans.
- `get_checksum(src_ip, dst_ip, tcp_packet)` — standalone checksum computation.

---

## UDP (`udp.rs`)

### Header Layout (8 bytes)

```
offset  size  field
  0      2    source_port
  2      2    dest_port
  4      2    length      (header + payload, big-endian)
  6      2    checksum    (0 = not computed, currently)
```

UDP checksum computation is implemented (`get_checksum`) but **not called** by `create_packet` — the checksum field is left as 0, which is legal for IPv4 UDP (RFC 768).

### API

- `create_packet(src_ip, dst_ip, src_port, dst_port, payload, out)` — writes 8-byte header + payload; returns byte count. Checksum is 0.
- `parse_packet(packet)` — returns `(src_port, dst_port, payload_slice)`.
- `get_checksum(src_ip, dst_ip, udp_packet)` — IPv4 pseudo-header checksum (available for callers that need it).

---

## SLIP (`slip.rs`)

SLIP (Serial Line IP, RFC 1055) is a byte-stuffing framing protocol that delimits IP packets over a serial link.

### Special Bytes

| Constant | Value | Meaning |
|----------|-------|---------|
| `SLIP_END` | `0xC0` | Frame delimiter |
| `SLIP_ESC` | `0xDB` | Escape prefix |
| `SLIP_ESC_END` | `0xDC` | Escaped `0xC0` |
| `SLIP_ESC_ESC` | `0xDD` | Escaped `0xDB` |

### Encoding

```
output = SLIP_END
for each byte b in input:
    if b == 0xC0: output += [SLIP_ESC, SLIP_ESC_END]
    if b == 0xDB: output += [SLIP_ESC, SLIP_ESC_ESC]
    else:         output += [b]
output += SLIP_END
```

### Decoding

Accumulate bytes until `SLIP_END` is seen with `out_pos > 0`; handle escape sequences. Returns `Some(len)` when a complete frame is received, `None` while still accumulating.

### Usage

`ipv4::send_packet` calls `slip::encode` then feeds each byte to `serial::write`. The receive path (`ipv4::receive_loop`) accumulates UART bytes and calls `slip::decode` on each new byte until a complete IP packet is decoded, then passes it to a callback.
