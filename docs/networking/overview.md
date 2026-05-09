# Architecture Overview

## Stack Layers

```
  Userland process
       │  syscalls 0x33–0x37
       ▼
  ┌───────────────────────────────────────┐
  │  Syscall dispatcher (abi/syscall.rs)  │
  └───────────────┬───────────────────────┘
                  │
       ┌──────────▼──────────┐
       │   netdrv.rs         │  routing table, driver/port registry
       └──────────┬──────────┘
                  │                        ┌──────────┐
        ┌─────────▼──────────┐             │  serial  │
        │   rtl8139.rs       │  PCI NIC    │  + SLIP  │  UART path
        └─────────┬──────────┘             └──────────┘
                  │
        ┌─────────▼───────────────────────────────────────┐
        │  Protocol helpers (stateless, no global state)  │
        │   ethernet.rs  arp.rs  ipv4.rs                  │
        │   icmp.rs  tcp.rs  udp.rs                       │
        └─────────────────────────────────────────────────┘
```

There are two independent paths:

| Path | Hardware | Protocol | Direction |
|------|----------|----------|-----------|
| **Ethernet** | RTL8139 PCI NIC | Ethernet II → IPv4/ARP | TX and RX |
| **Serial/SLIP** | UART COM1 | SLIP-framed IPv4 | TX only (active), RX (loop-based) |

![network-frame-routing](/assets/r2-network-frame-routing.png)

---

## Receive Path (Ethernet)

Frames arrive via polling, not IRQ. On every PIT tick (100 Hz) the scheduler calls `netdrv::poll_and_deliver()` before selecting the next runnable process:

```
PIT tick
  → scheduler_schedule()
    → netdrv::poll_and_deliver()
      → rtl8139::receive_frame()      read one frame from RX ring
      → tcp_dest_port()               extract TCP dest port (if IPv4/TCP)
      → lookup_port()                 find registered service pid
      → scheduler::push_msg(pid, msg) wake the target process
```

The frame is copied into `NET_FRAME_BUF` (2 KiB kernel static). The `Message` carries `buf_addr = NET_FRAME_BUF.as_ptr()` and `port_id = frame_len`. The receiving userland process calls syscall `0x35` which copies from `NET_FRAME_BUF` into a userland buffer. Because `NET_FRAME_BUF` is a single shared slot, only one frame is buffered at a time — the userland driver must consume each frame before the next tick.

## Transmit Path (Ethernet)

Userland calls syscall `0x34` with arg1 `0x04` (raw Ethernet) or `0x01` (IPv4):

```
syscall 0x34
  → derive frame length from EtherType / IP total_length field
  → rtl8139::send_frame(data, len)
    → copy into TX_BUFFERS[TX_INDEX]
    → write physical buffer address to TxAddr register
    → write send_len to TxStatus register
    → advance TX_INDEX (round-robin over 4 TX descriptors)
```

Minimum Ethernet frame size (60 bytes) is enforced by zero-padding in `send_frame`.

## Transmit Path (Serial/SLIP)

Userland calls `ipv4::send_packet`, which:

1. SLIP-encodes the IPv4 datagram (`slip::encode`).
2. Sends each encoded byte through the UART via `serial::write`.

This path is legacy/fallback; the RTL8139 path is preferred for QEMU guests.

## Same-Host Loopback

When `ipv4::send_packet` detects `src_ip == dst_ip` (same-guest delivery) it calls `netdrv::loopback_deliver` instead of going through the NIC. This copies the frame into `NET_FRAME_BUF` and pushes it to the target process's message queue directly, bypassing the serial encoder and the NIC TX/RX cycle.

---

## Driver and Port Registry

`netdrv.rs` maintains two static tables (no `Mutex` — both are written only at init time and read under the PIT tick):

| Table | Size | Contents |
|-------|------|----------|
| `NET_DRV_PID` | 1 entry | PID of the global Ethernet driver (ARP, ICMP, unbound TCP) |
| `PORT_REGISTRY` | 16 entries | `(tcp_dest_port, pid)` for TCP port-specific services |

### Registration (syscall `0x37`)

- `arg1 = 0`: register as global driver. Initialises the RTL8139, reads and caches the MAC address in `SYSTEM_CONFIG`. Idempotent — no-op if a driver is already registered.
- `arg1 = N > 0`: bind TCP destination port `N` to the calling process. If an entry for that port already exists it is updated (to support restart/handover). If the table is full, slot 0 is overwritten.

### Frame Routing

On each incoming frame `poll_and_deliver` calls `tcp_dest_port(frame)` to extract the TCP destination port (or `None` if not IPv4/TCP). It then calls `lookup_port(port)` against `PORT_REGISTRY`. If a match is found the frame goes to that service's PID; everything else (ARP, ICMP, unregistered ports) goes to `NET_DRV_PID`.

---

## Limits

| Resource | Value |
|----------|-------|
| RX ring buffer | 8 KiB + 1500-byte overrun guard |
| TX descriptors | 4 (round-robin) |
| TX buffer per descriptor | 2 KiB |
| Shared kernel frame buffer (`NET_FRAME_BUF`) | 2 KiB |
| Max port bindings | 16 |
| SLIP encode/decode buffer | 4 KiB |
| Serial baud rate | 38 400 (COM1, divisor 3) |
| Poll rate | 100 Hz (one frame per PIT tick) |
