# Hardware Layer

## PCI Enumeration (`pci.rs`)

Before a NIC can be used, its register base must be found by scanning the PCI configuration space. The kernel uses the standard x86 mechanism: write a configuration address to port `0xCF8`, then read data from `0xCFC`.

```
address = 0x8000_0000 | (bus << 16) | (device << 11) | (function << 8) | (offset & 0xFC)
```

`find_io_base(vendor_id, device_id)` iterates all 256 buses × 32 devices × 8 functions and returns the I/O BAR0 base (BAR0 bit 0 = 1 means I/O space). For RTL8139: vendor `0x10EC`, device `0x8139`. The RTL8139 is only initialised when PCI discovery finds it; the initial `0xC000` global value is not evidence of a card.

`enable_bus_mastering` sets bit 2 (Bus Master Enable) in PCI command register (offset `0x04`) so the NIC can DMA without CPU involvement.

---

## RTL8139 Driver (`rtl8139.rs`)

### Initialization (`rtl8139_init`)

Called once by `netdrv::register_driver` when the global Ethernet driver process registers via syscall `0x37`.

```
1. find_io_base()         discover I/O base from PCI BAR0
2. enable_bus_mastering() allow NIC DMA
3. CONFIG1 ← 0x00        power on
4. CMD ← 0x10            software reset; poll until bit 4 clears
5. RBSTART ← &RX_BUFFER  set RX ring base (physical == virtual, identity map)
6. CMD ← 0x0C            enable RX + TX
7. RCR ← 0xF | (1<<7) | (2<<11)   accept broadcast + multicast + runt frames; wrap mode, 32 KiB ring
8. IMR ← 0x0005          enable ROK (bit 0) and TOK (bit 2) interrupts
```

The RX buffer is a 32 KiB ring plus a 1 500-byte tail overrun guard (`RX_BUFFER: [u8; 32768 + 16 + 1500]`). The extra 16 bytes are the RTL8139 header prefix; the extra 1 500 are for frames that wrap across the ring boundary.

The driver does **not** register an IRQ handler. Instead, `receive_frame` is polled every PIT tick and acknowledges `ROK` in the ISR register (`+0x3E`) manually.

### RX Ring (`receive_frame`)

The RTL8139 writes incoming frames in order into `RX_BUFFER` starting at offset 0, wrapping around at 32 KiB. Each frame is prefixed with a 4-byte RTL8139 header:

```
offset +0: u16 rx_status  (bit 0 = ROK)
offset +2: u16 frame_len  (includes 4-byte CRC)
offset +4: frame data (frame_len bytes)
```

`receive_frame` checks `CAPR+16 != CBR` (not empty), reads the header at `RX_OFFSET & 0x7FFF`, copies `frame_len` bytes into the caller-supplied buffer, advances `RX_OFFSET` (aligned to 4 bytes), and writes the new read pointer to `CAPR` (`+0x38`) as `RX_OFFSET - 16` (RTL8139 datasheet quirk).

Frames shorter than 14 bytes or larger than the caller's buffer are dropped.

### TX Descriptors (`send_frame`)

The RTL8139 has 4 TX descriptors (TSD0–TSD3 at `+0x10`–`+0x1C`, TxAddr0–TxAddr3 at `+0x20`–`+0x2C`). The driver cycles through them round-robin via `TX_INDEX`.

```
1. copy data into TX_BUFFERS[TX_INDEX]  (zero-pad to 60 bytes minimum)
2. write physical buffer address to TxAddr[TX_INDEX]  (+0x20 + idx*4)
3. write frame length to TxStatus[TX_INDEX]           (+0x10 + idx*4)
   → writing TxStatus with length < 2048 starts the DMA
4. TX_INDEX = (TX_INDEX + 1) % 4
```

No completion poll: the driver reuses the four descriptors round robin without checking ownership. A burst of sends can therefore reuse a descriptor before transmission finishes.

### MAC Address (`read_mac_addr`)

The MAC is read from IDR0–IDR5 (registers `+0x00`–`+0x05`) after `rtl8139_init`. It is cached in `SYSTEM_CONFIG` so userland can query it without re-probing.

---

## NIC Selection and Intel Driver (`nic.rs`, `e1000.rs`)

`netdrv::register_driver` initialises the selected backend through `nic::init`. RTL8139 is tried first, then Intel; no supported card means registration fails. Routing and userland use the same interface for either backend: `peek_frame`, `consume_frame`, `send_frame`, MAC lookup and missed-packet counters.

The Intel driver recognises these PCI devices (vendor `0x8086`):

| Models | Device IDs |
|--------|------------|
| 82540EM, 82545EM | `0x100e`, `0x100f` |
| 82574L | `0x10d3` |
| 82577LM/LC, 82578DM/DC | `0x10ea`, `0x10eb`, `0x10ef`, `0x10f0` |
| 82579LM/V | `0x1502`, `0x1503` |

It uses BAR0 memory-mapped registers below 4 GiB and identity-mapped static DMA buffers. Legacy descriptor rings contain 32 RX and 64 TX descriptors, each with a 2 KiB buffer. No NIC interrupt handler is installed: timer polling handles receive delivery and link housekeeping. A full TX ring can refuse a send.

PCH controllers have additional reset, PHY, power-management and flash NVM setup. The kernel shell's [`nic`](../shell.md#nic) command exposes link state, PHY and ring diagnostics, reset, test transmission, register access and PCH NVM reads. Recognition of a model does not establish that every board's firmware configuration works.

For QEMU, replace `-device rtl8139,netdev=net0` with `-device e1000,netdev=net0` (82540EM) or `-device e1000e,netdev=net0` (82574L) to exercise the Intel backend.

---

## Serial / UART (`serial.rs`)

COM1 base address: `0x3F8`.

### Initialization

```
+1 ← 0x00   disable interrupts
+3 ← 0x80   enable DLAB (divisor latch access)
+0 ← 0x03   divisor low byte  → 38 400 baud  (115 200 / 3)
+1 ← 0x00   divisor high byte
+3 ← 0x03   8N1 (8 data bits, no parity, 1 stop bit)
+2 ← 0xC7   enable FIFO, clear TX+RX FIFOs, 14-byte threshold
+4 ← 0x0B   enable IRQs, assert RTS + DTR
```

### API

| Function | Description |
|----------|-------------|
| `init()` | Configure COM1 as above |
| `ready() → bool` | Check bit 0 of Line Status Register (`+5`); true if RX byte available |
| `read() → u8` | Read one byte from data register (`+0`) |
| `write(b: u8)` | Spin on LSR bit 5 (TX FIFO empty), then write to `+0` |

The UART is used as the transport for the SLIP path (see [protocols.md](protocols.md)).
