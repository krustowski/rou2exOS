# Overview

## Two Output Paths

The kernel supports two independent video paths selected at boot based on what the bootloader provides:

| Path | Source | Mode | Driver |
|------|--------|------|--------|
| **VESA framebuffer** | Multiboot2 framebuffer tag | Linear pixel buffer | `video/mode.rs` |
| **VGA text mode** | Always available | 80×25 character cells | `video/vga.rs` |

Both paths can be active simultaneously — the framebuffer is used by userland (pixel syscalls), while the VGA text buffer at `0xB8000` is used by the kernel itself for boot-status output.

---

## Boot-Time Detection

`init::check::init()` parses the Multiboot2 information structure passed by the bootloader. When a framebuffer tag (type 8) is found, the kernel fills `FRAMEBUFFER_PTR`:

```rust
pub static mut FRAMEBUFFER_PTR: FramebufferTag = FramebufferTag {
    addr:   0,
    pitch:  0,
    width:  0,
    height: 0,
    bpp:    0,
    fb_type: 0,
};
```

`FramebufferTag` mirrors the Multiboot2 tag layout:

| Field | Type | Description |
|-------|------|-------------|
| `addr` | `u64` | Physical address of the framebuffer |
| `pitch` | `u32` | Bytes per scan line |
| `width` | `u32` | Width in pixels |
| `height` | `u32` | Height in pixels |
| `bpp` | `u8` | Bits per pixel (16 or 32) |
| `fb_type` | `u8` | 1 = RGB, 2 = EGA text |

After parsing, `VIDEO_MODE` is set once and stays fixed for the lifetime of the kernel.

---

## VideoMode Enum (`video/mode.rs`)

```rust
pub enum VideoMode {
    Framebuffer {
        address: u64,
        pitch:   u32,
        width:   u32,
        height:  u32,
        bpp:     u8,
    },
    TextMode,
}

pub static mut VIDEO_MODE: Option<VideoMode> = None;
```

`VIDEO_MODE` is read by pixel-writing helpers and video syscalls to dispatch to the correct path.

---

## Pixel Writing

`put_pixel(x: u32, y: u32, r: u8, g: u8, b: u8)` in `video/mode.rs` writes a single pixel into the VESA framebuffer. The byte offset is:

```
offset = y * pitch + x * (bpp / 8)
```

Pixel encoding by `bpp`:

| bpp | Format | Encoding |
|-----|--------|----------|
| 32 | `0x00RRGGBB` | `(r as u32) << 16 \| (g as u32) << 8 \| b as u32` |
| 16 | RGB565 | `((r >> 3) << 11) \| ((g >> 2) << 5) \| (b >> 3)` |

Other bit depths are ignored (no-op).

---

## Text-Mode Character Writing

`put_char(x: u32, y: u32, ch: u8, color: u8)` writes a character cell to the VGA text buffer at `0xB8000`. Each cell is 2 bytes: character byte then attribute byte (color). The cell offset is:

```
offset = (y * 80 + x) * 2
```

This function is independent of the `Writer` struct — it is a direct write used by some low-level paths.

---

## Boot Status Display (`video/sysprint.rs`)

`sysprint.rs` provides a structured boot-status output layer that sits on top of the VGA text `Writer`. It maintains `SYSBUFFER: Mutex<Buffer>` — a 24-line display buffer (kept separate from the scrolling writer so status lines stay visible).

```rust
pub enum Result {
    Unknown,
    Passed,
    Failed,
    Skipped,
}
```

Each subsystem check (heap, processes, network, etc.) prints a line using `print_result(name, result)`. Output format:

```
[ OK    ]  kernel heap
[ FAIL  ]  some subsystem
[ SKIP  ]  optional feature
[ UNKNWN]  unverified
```

Colors are applied per `Result` variant using VGA attribute bytes.

---

## Video Syscalls

| Syscall | Number | Description |
|---------|--------|-------------|
| `put_pixel` | `0x12` | Write single pixel at (x, y) with RGB color |
| `blit_mode13` | `0x13` | Blit a 320×200 byte array into VGA mode 13h VRAM |
| `map_vram` | `0x14` | Map VGA VRAM (0xA00_000) into the calling process's page table |
| `set_video_mode` | `0x15` | Switch VGA hardware mode (0x03 / 0x0D / 0x12 / 0x13) |
| `get_fb_info` | `0x16` | Copy `FRAMEBUFFER_PTR` fields into userland-supplied buffer |
| `blit_buffer` | `0x17` | Copy userland pixel buffer into VESA framebuffer |
| `get_kernel_font` | `0x18` | Copy the embedded 8×16 CP850 font bitmap into userland buffer |

### Pointer Constraints

Userland pointers passed to video syscalls must fall in the validated range `0x600_000 – 0xA00_000`. Heap pointers (`0xC00_000+`) are rejected. Userland programs must stage data in their statically-allocated BSS or stack buffers.

The exception is `map_vram` (0x14), which maps `0xA00_000` into the calling process's page table using a P1 (4 KiB) sub-table allocated from `PAGE_TABLE_POOL`. After `map_vram`, the process can write to VGA VRAM directly at that virtual address without going through a syscall.

---

## Kernel Font (`VGA_FONT_8X16`)

A complete 8×16 pixel CP850 font (256 glyphs × 16 rows × 1 byte per row = 4096 bytes) is embedded as a static array in `video/vga_hw.rs`. It is used:

1. By `restore_font()` when switching back to VGA text mode (mode 0x03) — loaded into VGA plane 2 via planar write mode.
2. Via syscall `0x18` — returned to userland for software font rendering in pixel modes.
