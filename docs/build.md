# Build and Run

## Prerequisites

Run `make init` once to install the toolchain:

```
make init
```

This installs:

- Rust nightly via `rustup`
- Target `x86_64-unknown-none`
- Components `rust-src` and `llvm-tools-preview`
- `bootimage` cargo subcommand
- `grub2-mkrescue` must be present on the host (package: `grub2-tools-extra` or equivalent)
- `mtools` (`mmd`, `mcopy`) for floppy image creation

---

## Build

### Release build (both video modes)

```
make build
```

Produces `r2.iso`. Internally runs two cargo invocations:

| Feature flag | Output ELF | Description |
|---|---|---|
| `kernel_text` | `iso/boot/kernel_text.elf` | VGA text-mode path |
| `kernel_graphics` | `iso/boot/kernel_graphics.elf` | VESA framebuffer path |

Both ELFs are placed inside `iso/boot/`, then `grub2-mkrescue` assembles them into `r2.iso` with the modules `multiboot2 video video_bochs video_cirrus gfxterm all_video`.

### Debug build

```
make build_debug
```

Adds features `kernel_text,serial_debug`. Serial debug output is written to COM1 (`rprint!`/`rprintb!`/`rprintn!` macros). Note: serial debug disables SLIP networking (both use COM1).

Optional extra features:

```
make build_debug EXTRA_FEATURES=serial_debug
```

### Clean

```
make clean    # cargo clean
```

---

## Target Specification (`x86_64-r2.json`)

| Field | Value |
|---|---|
| `llvm-target` | `x86_64-unknown-none` |
| `os` | `none` |
| `linker` | `rust-lld` (LLD) |
| `relocation-model` | `static` |
| `panic-strategy` | `abort` |
| `disable-redzone` | `true` |
| `features` | `-mmx,-sse,+soft-float` |
| `exe-suffix` | `.elf` |

SSE is disabled at the target level; SSE is re-enabled at runtime by `init::cpu::enable_sse` (after the kernel stack is set up). The `soft-float` feature prevents LLVM from emitting SSE instructions before that point.

---

## Linker Script (`linker.ld`)

The kernel is linked starting at physical address `0x100000` (1 MiB).

| Region | Address / size | Notes |
|---|---|---|
| `.multiboot2_header` | `0x100000` (4 KiB aligned) | GRUB Multiboot2 header |
| `.text` | follows | All code |
| `.rodata` | follows | Read-only data, embedded fonts |
| `.data` + `.dma` | follows | Writable globals; `.dma` section holds the `DMA: [u8; 512]` floppy buffer at a known physical address |
| `.bss` | follows | Zero-initialised statics |
| `.gdt` | follows | GDT descriptor (assembly) |
| `.idt` | follows | IDT descriptor (assembly) |
| `__stack_bottom/top` | follows + 64 KiB | Kernel boot stack |
| `__heap_start/end` | follows + 64 KiB | Kernel linked-list heap |
| `p4_table` | 4 KiB aligned | PML4 page table |
| `p3_fb_table` | 4 KiB | P3 table for framebuffer mapping |
| `.user_task` | `0x650000` | Unused user-task section placeholder |
| `.dma` (DMA buffer) | `0x80000` (512-byte aligned) | Physical DMA target for ISA DMA channel 2 |

The `p2_table`, `p3_table`, `ist0/ist1_stack`, `tss64`, `multiboot_ptr`, and `debug_flag` symbols all live in assembly `.bss` in `boot.asm`.

---

## Boot Flow

```
GRUB (Multiboot2)
  │  loads r2.iso, selects kernel ELF
  │  passes Multiboot2 info pointer in EBX, magic in EAX
  ▼
_start  (boot.asm, 32-bit protected mode)
  ├── saves EBX/EAX → [multiboot_ptr] / [multiboot_magic]
  ├── loads P4 table address into CR3
  ├── set_up_page_tables()
  │     identity-maps 1 GiB via P2 (512 × 2 MiB huge pages)
  │     maps P3[1..5] → 4 × 1 GiB (addresses 1–4 GiB)
  │     marks P2[2..4] USER+WRITE (0x400000–0xA00000, userland range)
  ├── load_gdt()    — lgdt from gdt_descriptor
  ├── load_idt()    — lidt (empty; real IDT installed by init later)
  ├── set segment registers to data selector 0x10
  ├── enable_paging()
  │     CR4 bit 5 (PAE), EFER bit 8 (LME), CR0 bit 31 (PG)
  └── far jump to long_mode_entry (selector 0x08 = 64-bit code)
  ▼
long_mode_entry  (boot.asm, 64-bit long mode)
  ├── TLB flush (mov cr3, cr3)
  ├── set segment registers to 0x10
  ├── RSP ← __stack_top  (64 KiB kernel stack)
  └── call kernel_main(multiboot_magic, multiboot_ptr)
  ▼
kernel_main  (src/main.rs)
  ├── init::check::init(multiboot_ptr)   — 14-step boot sequence
  │     (see docs/init/overview.md)
  └── task::scheduler::idle(0xff)        — enters scheduler; never returns
```

### GDT Layout (from `boot.asm`)

| Selector | Descriptor | Description |
|---|---|---|
| `0x00` | null | Required null descriptor |
| `0x08` | `0x00AF9A000000FFFF` | Kernel code (64-bit, DPL=0) |
| `0x10` | `0x00AF92000000FFFF` | Kernel data (DPL=0) |
| `0x18` | `0x00affa000000ffff` | User code (64-bit, DPL=3) |
| `0x20` | `0x00aff2000000ffff` | User data (DPL=3) |
| `0x28` | TSS descriptor (patched at runtime) | 64-bit TSS |

The TSS descriptor at `0x28` is initially a placeholder; `init::idt::setup_tss_descriptor` overwrites it with the correct base address and limit before `ltr 0x28` is issued.

---

## Floppy Image (`make build_floppy`)

Creates a 1.44 MB FAT12 floppy image (`fat.img` by default, override with `FLOPPY_IMAGE=`):

```
make build_floppy
make build_floppy FLOPPY_IMAGE=my.img
```

1. `dd` creates a blank 2880-sector image.
2. `mkfs.fat -F 12` formats it.
3. `mmd` creates directories: `GARN`, `GFX`, `SLIP`, `SOUND`, `THEM`.
4. `mcopy` copies ELF binaries (from `../r2_app/`) and data files

The `INIT.RC` file is the startup script parsed by `init_rc` at boot (see below).

---

## Startup Script (`configs/init.rc`)

`INIT.RC` is read from the FAT12 root directory by the `init_rc` task during boot. Each non-blank, non-comment line is dispatched through `cmd::handle` — the same function used by the interactive shell.

Default `configs/init.rc`:

```sh
# Start network driver
bg ETH

# Start TNT with config
bg TNT eth

# Start the chatroom server
bg CHAT s eth

# Start GARN web server
bg GARN --config /mnt/fat/GARN/GARN.CFG

echo INIT.RC done
```

Lines starting with `#` are ignored. Trailing `\r` is stripped (DOS line endings tolerated).

---

## Run Targets

| Make target | Description |
|---|---|
| `make run_iso` | QEMU with CD-ROM only, 2 GB RAM, VGA std, serial PTY |
| `make run_iso_floppy` | + FAT12 floppy + PC speaker audio |
| `make run_iso_net` | + RTL8139 NIC on `tap0` + floppy + audio |
| `make run_iso_debug` | CD + floppy, serial → stdio, audio, no-reboot |
| `make run_iso_debug_int` | Same + `-d int,cpu_reset,page` (interrupt tracing) |
| `make run_iso_pty PTY_NUMBER=ptyN` | CD only, serial on specific PTY |
| `make run_iso_usb` | CD replaced by `/dev/sdb` (USB stick) |
| `make run_iso_floppy_drive` | Live floppy `/dev/sda` (requires sudo) |

Standard run with networking:

```
make run_iso_net
```

QEMU network setup assumes `tap0` is already created and bridged on the host. The kernel RTL8139 driver auto-detects the NIC via PCI scan.

```
sudo ip tuntap add dev tap0 mode tap
sudo ip link set tap0 up
sudo ip addr add 10.3.4.1/24 dev tap0
```

---

## Tests

### Kernel self-test (QEMU)

```
make test_kernel
```

Builds with features `kernel_test,kernel_text`. After running, QEMU exits via the `isa-debug-exit` device: exit code `33` means all tests passed, any other value means failure. Serial output goes to stdio.

### Host unit tests

```
make test
```

Compiles and runs `tests/unit/main.rs` as a standard Rust test binary on the host (no QEMU).

---

## Code Analysis

```
make clippy          # cargo clippy --release, all warnings as errors
make sonar_check     # SonarQube scan (requires SONAR_HOST_URL + SONAR_TOKEN env vars)
```
