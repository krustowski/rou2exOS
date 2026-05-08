# VGA

## Register Groups

VGA hardware is controlled through five groups of I/O-mapped registers. All access is through the kernel's `outb`/`inb` wrappers.

| Group | Index Port | Data Port | Purpose |
|-------|-----------|-----------|---------|
| Sequencer (SEQ) | `0x3C4` | `0x3C5` | Memory sequencing, clock, plane write masks |
| CRTC | `0x3D4` | `0x3D5` | Display timing, scan line counts, cursor position |
| Graphics Controller (GC) | `0x3CE` | `0x3CF` | Read/write modes, color compare, bit masks |
| Attribute Controller (AC) | `0x3C0` | `0x3C0` | Palette, display enable, pixel clock |
| Miscellaneous Output | `0x3C2` | — | Clock select, sync polarity, I/O address select |

The Attribute Controller uses a single port for both index and data, toggled by an internal flip-flop. It must be reset by reading `0x3DA` (Input Status 1) before writing.

---

## Supported Hardware Modes

`set_video_mode(mode: u8) -> bool` in `video/vga_hw.rs` programs all five register groups for the requested mode. Returns `true` on success, `false` for an unknown mode number.

| Mode | Number | Resolution | Colors | Use |
|------|--------|-----------|--------|-----|
| Text 80×25 | `0x03` | 720×400 (character) | 16 | Kernel console, default |
| 16-color 320×200 | `0x0D` | 320×200 | 16 | Legacy planar graphics |
| 16-color 640×480 | `0x12` | 640×480 | 16 | Higher-res planar graphics |
| 256-color 320×200 | `0x13` | 320×200 | 256 | Mode 13h, linear byte-per-pixel |

### Mode-Switch Sequence

For all modes:

1. Write Miscellaneous Output register (`0x3C2`).
2. Write Sequencer registers (`0x3C4`/`0x3C5`), index 0–4.
3. Unlock CRTC (clear protect bit on index `0x11`), then write CRTC registers.
4. Write Graphics Controller registers.
5. Reset AC flip-flop (read `0x3DA`), then write all 21 Attribute Controller registers (indices 0–20), finishing with a write of `0x20` to `0x3C0` to re-enable display output.
6. For mode `0x03` only: call `clear_text_buf()`, `restore_font()`, `restore_text_dac()`.

---

## Mode 0x03 — Text Mode Restoration

Switching back to text mode from a graphics mode requires three additional steps that reprogram state the BIOS would normally set up.

### `clear_text_buf()`

Fills the VGA text buffer at `0xB8000` with space characters (byte `0x20`) and attribute byte `0x07` (light gray on black). Clears all 80×25 = 2000 character cells.

### `restore_font()`

Reloads the embedded 8×16 font (`VGA_FONT_8X16`, 4096 bytes) into VGA plane 2 (the font plane). The sequence:

```
SEQ[0x00] ← 0x01   (synchronous reset)
SEQ[0x02] ← 0x04   (write to plane 2 only)
SEQ[0x04] ← 0x07   (sequential addressing, odd/even disabled)
GC[0x04]  ← 0x02   (read plane 2)
GC[0x05]  ← 0x00   (write mode 0, read mode 0)
GC[0x06]  ← 0x00   (A000-BFFF range, graph mode off)

for each glyph g (0..256):
    write 16 bytes of VGA_FONT_8X16[g*16..] to 0xA0000 + g*32

SEQ[0x02] ← 0x03   (restore write to planes 0+1)
SEQ[0x04] ← 0x03   (odd/even addressing)
GC[0x04]  ← 0x00   (read plane 0)
GC[0x05]  ← 0x10   (odd/even read)
GC[0x06]  ← 0x0E   (B800-BFFF range, text mode)
SEQ[0x00] ← 0x03   (release reset)
```

Each glyph occupies 32 bytes in plane 2 (only the first 16 are significant; the upper 16 are zero-padding required by VGA).

### `restore_text_dac()`

Programs the VGA DAC (Digital-to-Analog Converter) with the standard 16-color EGA-compatible palette. The DAC is accessed via:

- Write palette index to `0x3C8`
- Write R, G, B (6-bit, 0–63 each) sequentially to `0x3C9`

The standard palette maps the 16 EGA attribute colors (0–15) to their canonical RGB values (e.g., index 0 = black `0,0,0`; index 7 = light gray `42,42,42`; index 15 = white `63,63,63`).

---

## Text Mode Writer (`video/vga.rs`)

The `Writer` struct manages the kernel's VGA text console.

```rust
pub struct Writer {
    col_pos:    usize,
    row_pos:    usize,
    color_code: ColorCode,
    buffer:     *mut Buffer,   // 0xB8000
}
```

### Constants

| Constant | Value |
|----------|-------|
| `BUFFER_WIDTH` | 80 |
| `BUFFER_HEIGHT` | 25 |
| `BUFFER_ADDRESS` | `0xB8000` |

### Colors

`Color` is a 4-bit VGA attribute value (0–15). `ColorCode` packs foreground and background:

```
ColorCode = (background << 4) | foreground
```

Standard color indices: Black=0, Blue=1, Green=2, Cyan=3, Red=4, Magenta=5, Brown=6, LightGray=7, DarkGray=8, LightBlue=9, LightGreen=10, LightCyan=11, LightRed=12, Pink=13, Yellow=14, White=15.

### `write_byte(byte: u8)`

| Byte | Action |
|------|--------|
| `\n` (0x0A) | Advance to next row (`new_line()`) |
| `\r` / 0x08 | Backspace: decrement `col_pos`, write space, do not advance |
| Other | Write character cell at `(col_pos, row_pos)`, advance `col_pos`; call `new_line()` on column overflow |

### `new_line()`

When `row_pos` reaches `BUFFER_HEIGHT - 1`, the entire buffer is scrolled up one row: rows 1–24 are copied to rows 0–23 using direct memory writes to `0xB8000`. Row 24 is cleared. When `row_pos < BUFFER_HEIGHT - 1`, `row_pos` is incremented and `col_pos` is reset to 0.

### Hardware Cursor (`move_cursor()`)

The hardware text cursor is positioned by writing the linear cell index (`row * 80 + col`) as two bytes to CRTC:

```
CRTC[0x0E] ← (position >> 8) & 0xFF   (high byte)
CRTC[0x0F] ← position & 0xFF           (low byte)
```

Called after every character write that changes the cursor position.

---

## VGA VRAM Window

The VGA VRAM window (`0xA00_000` virtual, `0xA00_000` physical) is mapped on demand by syscall `0x14` (`map_vram`). The mapping uses fine-grained 4 KiB pages (a P1 table allocated from `PAGE_TABLE_POOL`) rather than the 2 MiB huge pages used elsewhere.

The P1 table is allocated once per process on the first `map_vram` call. Repeated calls are idempotent — the same P1 is reused. After mapping, userland can write directly to `0xA00_000` as Mode 13h VRAM (320×200, one byte per pixel, 64000 bytes).

The kernel's `TLB` is flushed by reloading CR3 after mapping.

---

## Legacy VGA Module (`src/vga/`)

An older VGA module lives under `src/vga/` (distinct from `src/video/`). It contains:

- `src/vga/buffer.rs` — character/color cell types
- `src/vga/write.rs` — writer implementation
- `src/vga/screen.rs` — screen helper

This module predates the current `video/` subsystem. The canonical implementation is `src/video/vga.rs`.
