# Video + Audio Output

## 0x10 (Print string)

Print provided string to terminal. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to char buffer | string length | ✅ |

## 0x11 (Clear the screen)

Effectively clear the text mode screen.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x00` | `0x00` | ✅ |

## 0x12 (Write graphical pixel)

Write a graphical pixel to the kernel framebuffer.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| encoded position | encoded color | ✅ |

## 0x13 (Write VGA buffer)

Write a VGA buffer into kernel framebuffer.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
|  a 320×200 VGA mode-13h palette-indexed buffer | pointer to RGB or default VGA palette | ✅ |

## 0x14 (Map VGA graphics RAM)

Maps physical VGA graphics RAM (`0xA0000–0xAFFFF`) into the calling process at virtual `0xA00_000` with USER+WRITE. 

On success writes `0xA00_000` into `*arg2`. Idempotent.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| reserved (`0x00`) | pointer to `uint64_t` — receives virtual base address | ✅ |

## 0x15 (Set VGA mode)

Programs VGA hardware registers for the given mode.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| video mode | reserved (`0x00`) | ✅ |

## 0x16 (VESA framebuffer geometry)

Get VESA framebuffer geometry. Writes `{ width, height, pitch, bpp }` into the struct pointed to by `arg1`. Returns `1` if no framebuffer is available, `0` on success.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to `FBInfo` struct | unused | ✅ |

## 0x17 (Blit VESA buffer)

Blit a 32bpp (`0x00RRGGBB`;) buffer to the VESA framebuffer. The kernel handles pitch mismatch. Scaled blit supported via encoded `arg2`.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to 32bpp pixel buffer | `0x00` for no scaling, or `(src_w << 16) | src_h` | ✅ |

## 0x18 (Copy kernel font)

Copy the kernel's embedded PSF1 glyph data to userland. 

Returns `char_size` (bytes per glyph = font height), or `0` on error. Glyph `n` occupies bytes `[n*char_size .. (n+1)*char_size]`; bit 7 (MSB) is the leftmost pixel.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to output buffer (`*mut u8`) | buffer capacity in bytes | ✅ |

## 0x1a (Play frequency)

Play given frequency in Hz for given time in milliseconds.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| frequency in Hz | length in milliseconds | ✅ |

## 0x1b (Play MIDI file)

Play the MIDI audio file.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x01` | pointer to the audio file | ✅ |

## 0x1f (Stop audio player)

Stop the player.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x00`| `0x00`| ✅ |