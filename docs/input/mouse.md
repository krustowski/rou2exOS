# Mouse

## Initialization (`mouse::init`)

`mouse::init()` must be called after the IDT is installed (so IRQ12 is handled). It configures the 8042 controller and the slave PIC to enable PS/2 mouse data reporting.

```
1. port 0x64 ← 0xA8          Enable Auxiliary Device (PS/2 port 2)

2. port 0x64 ← 0x20          "Read Controller Configuration Byte" (CCB)
   ccb ← port 0x60
   port 0x64 ← 0x60          "Write CCB"
   port 0x60 ← ccb | 0x02    Set bit 1: enable mouse IRQ12

3. port 0x21 &= ~0x04        Unmask IRQ2 on master PIC (enables all slave IRQs)
4. port 0xA1 &= ~0x10        Unmask IRQ12 on slave PIC

5. port 0x64 ← 0xD4          Route next byte to mouse
   port 0x60 ← 0xF4          Send 0xF4 (Enable Data Reporting) to mouse

6. discard ACK byte from port 0x60
```

Step 2 is required because QEMU initialises the CCB with bit 1 (mouse IRQ12 enable) cleared. Without it, the 8042 never asserts IRQ12 even if the mouse is sending data.

Step 3 is required because the cascade IRQ2 must be unmasked on the master PIC to allow any slave PIC interrupt (IRQ8–IRQ15) through. Without it, mouse clicks cause the Output Buffer Full bit (`0x64` bit 0) to become permanently set, blocking all further keyboard and mouse IRQs.

---

## IRQ12 Handler (`abi/idt.rs`)

On every byte from the PS/2 auxiliary port, the 8042 asserts IRQ12. `mouse_handler` fires at IDT vector `0x2C`:

1. Read status byte from port `0x64`.
2. Always read data byte from port `0x60` (must drain to keep OBF clear).
3. If `status & 0x20 != 0` (bit 5 set = auxiliary data): call `mouse::push_byte(data)`.
4. Acknowledge slave PIC: port `0xA0` ← `0x20`.
5. Acknowledge master PIC: port `0x20` ← `0x20`.

If bit 5 is clear, the byte was a keyboard byte or a spurious IRQ12 — it is discarded to avoid double-dispatch.

---

## 3-Byte Packet Reassembly

PS/2 mouse packets are 3 bytes long. The kernel reassembles them across consecutive IRQ12 firings using two static variables:

```
PHASE: usize    ← current byte index within the packet (0, 1, 2)
PKT_BUF: [u8; 3]
```

`push_byte(b)`:

1. **Sync check**: if `PHASE == 0` and bit 3 of `b` is not set → discard (sync lost). Byte 0 of a valid PS/2 packet always has bit 3 set.
2. Store `b` into `PKT_BUF[PHASE]`, increment `PHASE`.
3. When `PHASE == 3`: reset `PHASE = 0` and fan out the completed packet to all active subscribers.

### Packet Layout

```
Byte 0 (flags):
  bit 7: Y overflow
  bit 6: X overflow
  bit 5: Y sign (negative Δ)
  bit 4: X sign (negative Δ)
  bit 3: always 1 (used for sync)
  bit 2: Middle button
  bit 1: Right button
  bit 0: Left button

Byte 1: X displacement (signed, 9-bit when combined with bit 4 of byte 0)
Byte 2: Y displacement (signed, 9-bit when combined with bit 5 of byte 0)
```

Y axis convention: positive Δ = upward movement (opposite to screen coordinates).

---

## Subscriber Model (`input/mouse.rs`)

```rust
pub static mut MOUSE_RECEPTORS: [MouseSubscriber; 5]
```

Each `MouseSubscriber` holds:

| Field | Type | Description |
|-------|------|-------------|
| `pid` | `usize` | PID of the subscribed process (0 = free slot) |
| `pkts` | `[[u8; 3]; 64]` | Ring buffer of complete 3-byte packets |
| `head`/`tail` | `AtomicUsize` | Write/read indices (SeqCst) |

The ring can hold 64 packets (192 bytes). Overflow (ring full) silently drops the newest packet.

On completing a 3-byte packet, `push_byte` iterates all 5 slots and calls `push()` on every slot with `pid != 0`.

---

## Drain API

Userland reads mouse packets by calling `mouse_drain(pid, dst, max_bytes)`:

1. Find the subscriber slot for `pid`.
2. Call `drain_to(dst, max_pkts)`: copy whole 3-byte packets from the ring into `dst`, advance `tail`.
3. Returns bytes copied (always a multiple of 3).

`max_bytes` is divided by 3 to get `max_pkts`; the syscall passes `15` (5 packets maximum per call).

---

## Syscall Interface

| Syscall | Sub-opcode | Action |
|---------|-----------|--------|
| `0x03` | `0x04` | `mouse_subscribe()` — register calling process; finds first free slot |
| `0x03` | `0x05` | `mouse_drain(pid, arg2, 15)` — drain up to 5 packets into user buffer |
| `0x03` | `0x06` | `mouse_unsubscribe()` — deregister calling process |

`mouse_subscribe` ignores its `addr` argument (unlike the keyboard pipe, there is no mailbox path — all delivery is via the ring buffer and drain syscall).

Maximum 5 simultaneous mouse subscribers. Subscribing when all slots are full returns `-1`.

---

## Limits

| Resource | Value |
|----------|-------|
| Max subscribers | 5 |
| Packet ring size per subscriber | 64 packets (192 bytes) |
| Max bytes per drain call | 15 (5 × 3-byte packets) |
| Packet reassembly buffer | 3 bytes (global, shared across all subscribers) |
