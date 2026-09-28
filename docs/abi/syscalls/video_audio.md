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

Write a graphical pixel to the VESA framebuffer.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `(x << 16) \| y` — pixel coordinates | `0x00RRGGBB` — 24-bit RGB color | ✅ |

## 0x13 (Write VGA buffer)

Blit a 320×200 palette-indexed buffer into the VESA framebuffer. Each pixel is expanded to 32bpp using the supplied palette, or the default VGA 256-color palette if none is given.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to 64000-byte palette-indexed buffer (320×200, 1 byte per pixel) | pointer to 768-byte palette (256 × RGB triplets), or `0` to use the default VGA palette | ✅ |

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

## 0x19 (Blit an 8-bit indexed frame)

Draw a frame of palette indices, one byte a pixel, scaled by the largest whole number that fits and centred on the framebuffer, converting to the framebuffer's own format (32, 24 or 16 bpp) on the way. The palette is 256 `(r, g, b)` byte triples. Only rows `first_row .. first_row + rows` are sent, so a program can update a band of the screen.

`arg1` points to:

```c
struct IndexedFrame {       /* packed */
    uint64_t pixels;        /* width * height palette indices */
    uint64_t palette;       /* 256 * 3 bytes */
    uint32_t width, height; /* at most the framebuffer's size */
    uint32_t first_row, rows;
};
```

With `arg1 = 0` the call only answers whether there is anything to draw on: `0` when GRUB set up an RGB framebuffer (the graphics kernel), `1` when not (the text kernel, whose "framebuffer" is the text console). Otherwise returns `0` on success, `1` without an RGB framebuffer, or `InvalidInput` for a frame that does not fit or buffers that are not the caller's. `arg2 = 1` clears the whole screen to black first.

Memento's r2 backend uses this on the graphics kernel: its screen is already one byte a pixel, so it shows 256 colours without keeping a 32-bit copy of the screen.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to `IndexedFrame`, or `0` to ask | `1` to clear first, else `0` | ✅ |

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

Play a Standard MIDI File from the filesystem on the PC speaker. The call blocks until the song ends.

Formats 0, 1 and 2 are accepted (format 1 tracks are merged by time, format 2 patterns play back to back), including tempo changes and SMPTE time divisions. The speaker is monophonic, so the highest note held at any moment is the one voiced; channel 10 (percussion) is ignored. The name is an absolute path or one relative to the working directory, on the floppy or on a read-only mount (`/mnt/iso`, `/mnt/tar`). Files are read into a 4 KiB buffer: from the floppy a longer file plays truncated, from a read-only mount it is refused with `InvalidInput`. Returns `InvalidInput` also when the file is not a valid MIDI file.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x01` (Standard MIDI File, format 0/1/2) | pointer to NUL-terminated file name | ✅ |

## 0x1f (Stop audio player)

Stop the player.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x00`| `0x00`| ✅ |

## 0x3f (HD Audio PCM)

Sound through the Intel HD Audio controller: 16-bit signed stereo PCM, left then right, into a 128 KiB ring the kernel plays round and round (about 0.68 s at 48 kHz). Nothing waits: a write takes as many whole samples as there is room for. A stream that runs dry plays silence. See [HD Audio](../../audio/hda.md) for the driver.

| Argument 1 | Argument 2 | Returns |
|------------|------------|---------|
| `0x01` open | rate in Hz: 8000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 88200 or 96000 | `0x00`; `NotImplemented` without a controller; `InvalidInput` for another rate. Takes the stream over from whoever had it |
| `0x02` write | pointer to `{ buffer: u64, length: u64 }` | the bytes taken (0 when the ring is full or nothing is open); `u64::MAX` for a pointer outside the user regions |
| `0x03` queued | *unused* | bytes queued and not yet played |
| `0x04` close | *unused* | `0x00` (only the program that opened it closes it) |
| `0x05` pause | *unused* | `0x00`: the DMA stops, the ring and the position stay |
| `0x06` resume | *unused* | `0x00` |

libc++r2 wraps it in `r2/audio.hpp` (`r2::audio::open`, `write`, `queued`, `close`, `pause`, `resume`).
