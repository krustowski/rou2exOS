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
| 5 | `parser::parse_info(m2_ptr, ...)` | `init/parser.rs` | Parse Multiboot2 tags; fill `FRAMEBUFFER_PTR`; sum usable RAM (`total_ram_bytes`); keep the kernel command line |
| 6 | `usb::take_all()` | `usb/mod.rs` | Only with `usb=os` on the command line: take the USB controllers from the firmware (see [Taking USB from the Firmware](../build.md#taking-usb-from-the-firmware)) |
| 7 | `mouse::init()` | `input/mouse.rs` | Enable PS/2 aux port and IRQ12 |
| 8 | `heap::pmm_heap_init()` | `init/heap.rs` | Init kernel linked-list heap; run smoke test |
| 9 | `video::print_result(...)` | `init/video.rs` | Call `init_video(fb)` to set `VIDEO_MODE` |
| 10 | `fs::floppy_check_init()` | `init/fs.rs` | Probe FAT12 floppy; set cwd to `/` |
| 11 | `fs::vfs_init(fat12)` | `init/fs.rs` | Mount `/`, `/mnt/fat` (only if step 10 found a FAT12 volume), `/mnt/tmp` (the RAM disk, placed in high memory and formatted FAT16), `/mnt/iso` (if CD present), `/mnt/tar` (if GRUB loaded the archive) |
| 12 | `color::color_demo()` | `init/color.rs` | Print 16-color swatch to console |
| 13 | `ascii::ascii_art()` | `init/ascii.rs` | Print kernel splash text |
| 14 | `process::init_processes()` | `init/process.rs` | Save CR3, init userland heap, create initial tasks |
| 15 | `debug::dump_debug_log_to_memdisk()` | `debug.rs` | Write the debug log so far to `/mnt/tmp/KERNDBG.LOG` |
| 16 | `pit::pic_pit_init()` | `init/pit.rs` | Remap 8259A PIC; start PIT at 1000 Hz; `sti` |

Step 16 (`sti`) is the point of no return — from here the PIT fires every 1 ms and the scheduler takes over. `init` never runs again.

---

## Debug Log

`debug!`, `debugn!` and `debugln!` (`src/debug.rs`) append to `DEBUG_LOG`, an 8 KiB buffer in memory; what does not fit once it is full is dropped. The kernel writes it out at three points:

| When | Where |
|------|-------|
| The Multiboot2 framebuffer tag is parsed (step 6) | `DEBUG.TXT` in the floppy's root, and the serial port (COM1) |
| End of init (step 14) | `/mnt/tmp/KERNDBG.LOG` on the RAM disk |
| The shell's [`debug`](../shell.md#debug-hidden) command | `DEBUG.TXT`, `KERNDBG.LOG` and serial, each file replaced |

`KERNDBG.LOG` is there for a machine with neither a floppy nor a serial line, such as one booted from a USB stick: `read /mnt/tmp/KERNDBG.LOG` shows it. It is written before the timer starts, while no process can be writing to the RAM disk, so it holds everything up to that point and nothing logged afterwards until `debug` is run.

The write at step 6 comes before the floppy probe, and on a machine without a floppy it prints `ERR: Could not find the FAT12 label, floppy may not be present` on the boot screen. It is harmless: the log still reaches serial and, later, the RAM disk.

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
| `pit.rs` | 8259A PIC remap, PIT init at `TICKS_PER_SECOND` (1000 Hz) |
| `heap.rs` | Kernel heap init + smoke test |
| `fs.rs` | Floppy probe, VFS mount table init |
| `video.rs` | `init_video()`, optional VESA P1 mapping |
| `process.rs` | Initial task creation (kmain, init_rc, kclock, kshell; the last two not on a framebuffer) |
| `config.rs` | `SYSTEM_CONFIG` global, `get_prompt()` |
| `font.rs` | PSF1/PSF2 font parser, `PSF_FONT` static, glyph renderer |
| `ascii.rs` | Splash screen text |
| `color.rs` | 16-color palette swatch |
