# Subsystems

---

## CPU (`init/cpu.rs`)

`cpu::check()` enables the two CPU features the kernel requires before anything else, then returns `Result::Passed`.

### SSE (`enable_sse`)

```
CR4 |= (1 << 9)   OSFXSR   — allow FXSAVE/FXRSTOR and SSE instructions in OS context
CR4 |= (1 << 10)  OSXMMEXCPT — allow OS to handle SSE exceptions
CR0 &= !(1 << 2)  Clear EM  — disable x87 emulation
CR0 |= (1 << 1)   Set MP    — monitor co-processor
```

Without this, any SSE instruction (including Rust's auto-vectorised memory copies) triggers `#UD`.

### SYSCALL/SYSRET (`enable_syscalls`)

Sets up the `syscall` instruction path via two MSRs:

| MSR | Address | Value |
|-----|---------|-------|
| `IA32_EFER` | `0xC0000080` | Set bit 0 (SCE — Syscall Enable) |
| `IA32_LSTAR` | `0xC0000082` | Address of `syscall_handler` stub |

The `syscall_handler` stub itself is currently a `swapgs` / `sysretq` skeleton — the kernel uses interrupt `0x7F` as its actual syscall gate, not `syscall`.

---

## GDT, IDT, TSS (`init/idt.rs`)

`idt_isrs_init()` performs five sequential steps:

### 1. Install ISRs (`install_isrs`)

Registers handlers in the `IDT` static (see `abi/idt.rs`): `#UD`, `#DF`, `#GP`, `#PF`, timer (PIT), keyboard (IRQ1), floppy (IRQ6), mouse (IRQ12), syscall gate (`0x7F`).

### 2. Load IDT (`load_idt`)

Issues `lidt` with the 10-byte IDTR pointing at the `IDT` static.

### 3. Initialise TSS (`init_tss`)

Zeros the 104-byte `Tss64` struct then fills three pointer fields:

| Field | Value | Purpose |
|-------|-------|---------|
| `rsp0` | address of `__stack_top` (linker symbol) | Ring-0 stack for hardware interrupt entry from ring 3 |
| `ist1` | address of `ist0_stack_top` | IST stack for `#DF` (double fault) |
| `ist2` | address of `ist1_stack_top` | IST stack for `#PF` (page fault) |

`io_map_base` is set to `sizeof(Tss64)` to disable the I/O permission bitmap (all ports allowed from ring 0).

### 4. Write TSS descriptor (`setup_tss_descriptor`)

Builds a 16-byte x86-64 TSS descriptor (two consecutive GDT slots) and writes it into the `gdt_tss_descriptor` label in the assembly-defined GDT:

- Type `0x9` (available 64-bit TSS), present, DPL=0
- Base: full 64-bit address of `tss64`
- Limit: `0x67` (104 bytes − 1)

### 5. Reload GDT and TSS (`reload_gdt`, `load_tss`)

`lgdt` reloads the GDTR from `gdt_start/gdt_end` linker symbols. `ltr 0x28` loads the TSS from GDT selector `0x28`.

---

## PIC and PIT (`init/pit.rs`)

`pic_pit_init()` performs both steps then returns.

### PIC Remap (`remap_pic`)

The BIOS maps IRQs 0–15 to IDT vectors 0–15, which overlap CPU exceptions. `remap_pic` reinitialises both 8259A PICs (master and slave) via the standard 4-word ICW sequence:

```
ICW1 (0x11): init + ICW4 needed
ICW2:        master offset = 0x20 (IRQs 0–7  → vectors 0x20–0x27)
             slave  offset = 0x28 (IRQs 8–15 → vectors 0x28–0x2F)
ICW3:        master: slave on IRQ2 (0x04)
             slave:  cascade identity (0x02)
ICW4 (0x01): 8086/88 mode
```

`io_wait()` writes a no-op byte to port `0x80` between each word to give the PIC time to process the command.

Existing IMR masks are saved before and restored after, so no IRQs are inadvertently unmasked.

### PIT Init (`init_pit`)

Programs the 8253/8254 Programmable Interval Timer channel 0 in mode 3 (square wave):

```
port 0x43 ← 0x36      channel 0, lobyte/hibyte, mode 3, binary
divisor = 1_193_182 / frequency_hz    (= 11931 for 100 Hz)
port 0x40 ← divisor & 0xFF            low byte
port 0x40 ← (divisor >> 8) & 0xFF    high byte
sti                                    enable interrupts
```

`TICKS_PER_SECOND = 100` (10 ms per tick). The tick counter is maintained in `time/acpi.rs`.

---

## Multiboot2 Parsing (`init/boot.rs`, `init/parser.rs`)

`parser::parse_info(m2_ptr, fb_tag)` is a thin wrapper that calls `boot::parse_multiboot2_info` and maps the return value (tag count) to `Result::Passed/Failed`.

`parse_multiboot2_info(base_addr, fb_tag)` walks the Multiboot2 information structure:

- Reads `total_size` from the first 4 bytes, then iterates over 8-byte-aligned `TagHeader` records until type `0` (end tag) or 64 tags processed.
- Tags handled:

| Type | Content |
|------|---------|
| `1` | Boot command line (logged to debug) |
| `3` | Module tag (logged to debug) |
| `6` | Memory map — iterates entries, logs usable regions (type 1) |
| `8` | Framebuffer — copies the tag into `fb_tag`; draws debug rectangles and PSF text to the framebuffer |
| `14` | ACPI v1 RSDP (logged) |

The framebuffer tag (type 8) is the only one that produces a side-effect beyond logging: it fills `FRAMEBUFFER_PTR`, which is subsequently used by all video syscalls.

### `FramebufferTag` struct (`#[repr(C, packed)]`)

| Field | Type | Description |
|-------|------|-------------|
| `addr` | `u64` | Physical address of the framebuffer |
| `pitch` | `u32` | Bytes per scan line |
| `width` | `u32` | Width in pixels |
| `height` | `u32` | Height in pixels |
| `bpp` | `u8` | Bits per pixel |
| `fb_type` | `u8` | 1 = RGB, 2 = EGA text |

---

## Heap Init (`init/heap.rs`)

`pmm_heap_init()` initialises the kernel linked-list heap (`mem/heap.rs`) and runs a smoke test:

1. `mem::heap::init()` — writes the single free `HeapNode` spanning `__heap_start–__heap_end` (64 KiB).
2. Runs 3 identical cycles: allocate blocks of 5, 50, and 500 bytes; validate each returned address is within `[__heap_start, __heap_end]`; free all three.

If any allocation falls outside the heap range, the function returns `Result::Failed` immediately.

`init_heap_allocator()` (unused at runtime) is an alternative path that would initialise the bump allocator (`BumpAllocator`) instead.

---

## Filesystem Init (`init/fs.rs`)

### `floppy_check_init()`

1. Calls `Floppy::init()` and attempts to open the FAT12 filesystem.
2. Returns `Result::Passed` on success, `Result::Skipped` on failure (floppy missing is non-fatal).
3. Always sets `SYSTEM_CONFIG` path to `b"/"` cluster `0` regardless of outcome.

### `vfs_init()`

Populates the VFS mount table with three entries:

| Path | FS type | Notes |
|------|---------|-------|
| `/` | `Root` | Always mounted |
| `/mnt/fat` | `Fat12` | Always mounted; operations fail gracefully if floppy absent |
| `/mnt/iso` | `Iso9660` | Mounted only if `Iso9660::probe()` succeeds (CD present) |

---

## Video Init (`init/video.rs`)

`video::print_result(fb)` calls `video::mode::init_video(fb)`:
- If `fb.addr != 0`: sets `VIDEO_MODE = Some(VideoMode::Framebuffer { ... })`.
- Otherwise: sets `VIDEO_MODE = Some(VideoMode::TextMode)`.

Returns `Result::Passed` if `VIDEO_MODE` is `Some` after the call.

`map_framebuffer(...)` is an unused helper that would walk the P4/P3/P2/P1 page table hierarchy to map a physical framebuffer at an arbitrary virtual address using 4 KiB pages. The kernel currently uses the identity-mapped physical address directly.

---

## Process Init (`init/process.rs`)

`init_processes()` is called as the last step before the PIT/scheduler are started. It runs once, then the scheduler takes over permanently.

```
init_processes()
  ├── mem::pages::save_kernel_cr3()    snapshot current CR3
  ├── mem::uheap::init()               map 0xC00_000–0xFFF_FFF USER+WRITE
  └── setup_processes()
        ├── new_process("kmain",  kernel_idle,  ...)   slot 0 — boot RSP sentinel
        ├── new_process("init_rc", init_rc,     ...)   slot 1 — startup script
        ├── new_process("clock",  clock_test,   ...)   slot 2 — RTC clock display
        └── new_process("shell",  keyboard_loop, ...)  slot 3 — interactive shell
              set_shell_pid(shell_pid)
```

### Initial Tasks

**`kernel_idle` (slot 0):** Absorbs the kernel's boot-time RSP on the first PIT tick (the scheduler saves the current RSP into `slot 0` before switching away). Loops on `hlt` forever. Required as a sentinel — without it the scheduler's first context save would corrupt the iretq frame of a real process.

**`init_rc` (slot 1):** Reads `INIT.RC` from FAT12 root directory. Parses it line by line (NUL or `\n` terminated; strips trailing `\r`; ignores blank lines and lines starting with `#`). Each non-comment line is dispatched through `cmd::handle()` — the same function used by the interactive shell. After the file is fully processed the task kills itself and loops on `hlt`.

**`clock_test` (slot 2):** Reads the RTC (`h:m:s`) in a tight poll loop and renders the time to a fixed VGA text position. Uses the legacy `vga/write.rs` module (separate from `video/vga.rs`).

**`shell` (slot 3):** Runs `keyboard_loop()` — the interactive kernel shell. This task runs for the lifetime of the kernel. Its PID is saved in the scheduler via `set_shell_pid` so that `syscall 0x00` (exit) can wake it when a child process terminates.

---

## System Configuration (`init/config.rs`)

`SYSTEM_CONFIG: Mutex<SystemConfig>` is the kernel's global runtime state store, accessible from any subsystem.

### `SystemConfig` Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `user` | `[u8; 32]` | `"root"` | Current username (space-padded) |
| `host` | `[u8; 32]` | `"rourex"` | Hostname (space-padded) |
| `path` | `[u8; 32]` | `"/"` | Current working directory string |
| `path_len` | `usize` | `1` | Byte length of the path string |
| `path_cluster` | `u16` | `0` | FAT12 cluster for cwd (`0` = root or ISO9660) |
| `version` | `[u8; 16]` | `"v0.11.4"` | Kernel version string |
| `ip_addr` | `[u8; 4]` | `[0,0,0,0]` | IPv4 address (set by ETH driver via syscall 0x01) |
| `mac_addr` | `[u8; 6]` | zeros | MAC address (set by RTL8139 init) |

All string fields use trailing space padding. Getters (`get_user`, `get_host`, `get_path`, `get_version`) return a trimmed slice with trailing spaces removed.

### `get_prompt()`

Assembles `user@host:path > ` into a static 80-byte buffer `PROMPT_BUF` and returns a slice. Falls back to `"$ "` if the config lock is contended.

### Linker Symbol Exports

`config.rs` also declares `extern "C"` links to assembly/linker symbols:

| Symbol | Type | Description |
|--------|------|-------------|
| `p4_table` | `[u64; 512]` | Boot PML4 table |
| `p3_table`, `p2_table` | `u64` | Boot P3/P2 table addresses |
| `p3_fb_table`, `p2_fb_table`, `p1_fb_table`, `p1_fb_table_2` | `[u64; 512]` | Framebuffer page table statics |
| `multiboot_ptr` | `u32` | Multiboot2 info pointer (from boot assembly) |
| `debug_flag` | `u8` | Non-zero enables serial debug output |
| `__stack_start`, `__stack_end` | `u8` | Boot stack bounds (linker symbols) |

---

## PSF Font (`init/font.rs`)

`PSF_FONT: &[u8]` is the entire `terminus-font.psf` file embedded at compile time via `include_bytes!`.

`parse_psf(psf)` returns a `PsfFont<'_>` by detecting the magic bytes:

| Magic | Format | Header parse |
|-------|--------|-------------|
| `0x36 0x04` | PSF1 | `bytes_per_glyph = psf[3]`; glyphs start at byte 4 |
| `0x72 0xB5 0x4A 0x86` | PSF2 | Full 32-byte header; reads `header_len`, `glyph_size`, `height`, `width` |

`draw_char_psf(font, ch, x, y, color, fb, pitch, bpp)` renders one glyph by iterating rows and testing each bit (MSB = leftmost pixel). `draw_text_psf(text, font, x, y, ...)` renders a string by advancing `x` by `font.width` per character.

The same font data is returned to userland via syscall `0x18` (glyph data starting at byte 4).

---

## Splash (`init/ascii.rs`, `init/color.rs`)

**`ascii_art()`** — prints the kernel logo in green, then resets to white:

```
                 ____            ___  _____
 _ __ ___  _   _|___ \ _____  __/ _ \/ ____|
| '__/ _ \| | | | __) / _ \ \/ / | | \___ \
| | | (_) | |_| |/ __/  __/>  <| |_| |___) |
|_|  \___/ \__,_|_____\___/_/\_\____/|____/
```

**`color_demo()`** — iterates VGA color attributes 0–15, prints two-space blocks using each as the background color, arranged in two rows of 8, to confirm the palette is functional.
