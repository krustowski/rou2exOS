# Overview

The `init` module is the kernel's sequential startup orchestrator. It is entered once, runs each subsystem check in order, prints a status line for each step, then hands off to the scheduler — at which point it never runs again.

The single entry point is `init::check::init(m2_ptr: u32)`, called from `main.rs` with the Multiboot2 info pointer left in a register by the bootloader.

---

## Boot Sequence

```
main(m2_ptr)
  └── init::check::init(m2_ptr)
```

Steps execute in this exact order:

| # | Call | Module | Description |
|---|------|--------|-------------|
| 1 | `vga::init_writer()` | `video/vga.rs` | Create the VGA text `Writer` at `0xB8000` |
| 2 | `clear_screen!()` | macro | Blank the 80×25 text buffer |
| 3 | `cpu::check()` | `init/cpu.rs` | Enable SSE (CR4/CR0), set up SYSCALL/SYSRET MSRs |
| 4 | `idt::idt_isrs_init()` | `init/idt.rs` | Install ISRs, reload GDT, init TSS, load IDT |
| 5 | `mouse::init()` | `input/mouse.rs` | Enable PS/2 aux port and IRQ12 |
| 6 | `parser::parse_info(m2_ptr, ...)` | `init/parser.rs` | Parse Multiboot2 tags; fill `FRAMEBUFFER_PTR` |
| 7 | `heap::pmm_heap_init()` | `init/heap.rs` | Init kernel linked-list heap; run smoke test |
| 8 | `video::print_result(...)` | `init/video.rs` | Call `init_video(fb)` to set `VIDEO_MODE` |
| 9 | `fs::floppy_check_init()` | `init/fs.rs` | Probe FAT12 floppy; set cwd to `/` |
| 10 | `fs::vfs_init()` | `init/fs.rs` | Mount `/`, `/mnt/fat`, `/mnt/iso` (if CD present) |
| 11 | `color::color_demo()` | `init/color.rs` | Print 16-color swatch to console |
| 12 | `ascii::ascii_art()` | `init/ascii.rs` | Print kernel splash text |
| 13 | `process::init_processes()` | `init/process.rs` | Save CR3, init userland heap, create initial tasks |
| 14 | `pit::pic_pit_init()` | `init/pit.rs` | Remap 8259A PIC; start PIT at 100 Hz; `sti` |

Step 14 (`sti`) is the point of no return — from here the PIT fires every 10 ms and the scheduler takes over. `init` never runs again.

---

## Global State Set During Init

| Symbol | Type | Set by step | Description |
|--------|------|-------------|-------------|
| `FRAMEBUFFER_PTR` | `boot::FramebufferTag` | 6 | VESA framebuffer address, pitch, dimensions, bpp |
| `VIDEO_MODE` | `Option<VideoMode>` | 8 | Active video path (Framebuffer or TextMode) |
| `SYSTEM_CONFIG` | `Mutex<SystemConfig>` | 9 | hostname, user, cwd, version, IP, MAC |
| `KERNEL_CR3` | `u64` | 13 | Boot-time page table snapshot for process cloning |
| Userland heap P2[6/7] | page table | 13 | `0xC00_000–0xFFF_FFF` mapped USER+WRITE |
| `SCHEDULER` | `Mutex<Scheduler>` | 13 | Initial process slots populated |

---

## Module Table

| File | Purpose |
|------|---------|
| `check.rs` | Entry point `init()`, `FRAMEBUFFER_PTR` global |
| `boot.rs` | Multiboot2 tag structs, `parse_multiboot2_info` |
| `parser.rs` | Thin `Result`-returning wrapper around `boot::parse_multiboot2_info` |
| `cpu.rs` | SSE enable, SYSCALL/SYSRET MSR setup |
| `idt.rs` | GDT reload, TSS init, IDT load |
| `pit.rs` | 8259A PIC remap, PIT 100 Hz init |
| `heap.rs` | Kernel heap init + smoke test |
| `fs.rs` | Floppy probe, VFS mount table init |
| `video.rs` | `init_video()`, optional VESA P1 mapping |
| `process.rs` | Initial task creation (kmain, init_rc, clock, shell) |
| `config.rs` | `SYSTEM_CONFIG` global, `get_prompt()` |
| `font.rs` | PSF1/PSF2 font parser, `PSF_FONT` static, glyph renderer |
| `ascii.rs` | Splash screen text |
| `color.rs` | 16-color palette swatch |
