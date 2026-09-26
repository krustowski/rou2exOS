# SDK Overview

Software development kit for writing userland programs for the `r2` kernel.

The SDK lives in the [rou2exOS-apps](https://github.com/krustowski/rou2exOS-apps) repository, next to the programs built with it. Every language has its own workspace in the repository root, and three of them carry a library that implements the [kernel ABI](../abi/syscall_specification.md) so an application does not have to:

| Language | Library | What it gives you | Page |
|----------|---------|-------------------|------|
| C | `c/libcr2` | Syscall wrappers, `printf`, strings, a TCP/IP stack (ARP, ICMP, DHCP, TCP) | [libcr2](libcr2.md) |
| C++ | `cpp/libc++r2` | A freestanding C++23 runtime and standard library: containers, strings, formatted output, `expected`, coroutines, filesystem, graphics, input | [libc++r2](libcxxr2.md) |
| Go (TinyGo) | `go/libgor2`, `go/r2net`, `go/tinygo-r2` | The syscall binding, a TCP/IP stack with DNS and an HTTP/1.0 client, and the TinyGo target that makes Go run at all | [libgor2](libgor2.md) |
| NASM, Rust | — | Minimal examples calling `int 0x7f` directly | — |

The programs shipped with the kernel are listed on the [Application Suite](apps.md) page.

---

## Repository Layout

```
rou2exOS-apps/
├── c/
│   ├── libcr2/          the C library (+ _crt0.asm)
│   ├── linker.ld        shared linker script for C programs
│   ├── Makefile.tmpl    shared build rules for C programs
│   └── <app>/           one directory per program
├── cpp/
│   ├── libc++r2/        the C++ runtime and standard library
│   ├── example-print/   the minimal C++ program, no library
│   └── memento-hello/   the Memento GUI demo
├── go/
│   ├── libgor2/         the syscall binding
│   ├── r2net/           the TCP/IP stack
│   ├── tinygo-r2/       the TinyGo target (runtime hooks, r2.ld, r2.S)
│   ├── Makefile.tmpl    shared build rules for Go programs
│   └── <app>/           one directory per program
├── nasm/
└── rust/
```

The kernel's `Makefile` expects this repository to be checked out as `../r2_app` next to the kernel tree: `make build` copies the built binaries into `iso/bin/`, and `make build_floppy` copies data files and some binaries onto `fat.img`.

---

## What Every Program Has to Respect

These rules come from the kernel, not from any one library, and apply whichever language a program is written in.

### Executable format

Programs are statically linked `elf64-x86-64` executables with a `.ELF` extension. The kernel loads `PT_LOAD` segments only; there is no dynamic linker, no relocation, and no C runtime in the kernel, so every program brings its own start-up code (`_crt0.asm` in C and C++, `r2.S` in Go).

On entry the kernel leaves a System V argv frame on the initial stack:

```
[rsp+0]   argc
[rsp+8]   argv[0]
[rsp+16]  argv[1]
...
```

The initial stack is small and sits in memory shared with other processes, so every runtime reads `argc`/`argv` and immediately switches to a private stack inside its own image.

### Memory map

Each process gets a **private 2 MiB frame mapped at `0x600_000`**, which has to hold code, data, `.bss`, the private stack and (for C++ and Go) the heap. The kernel refuses to load a segment that ends past `0xA00_000`.

```
0x600_000  ┌──────────────────────────────┐
           │ .text .rodata .data .bss     │  private to this process
           │ private stack, private heap  │
0x800_000  ├──────────────────────────────┤  <-- stay below this line
           │ other processes' initial     │  shared, identity-mapped
           │ stacks (per-slot table)      │
0xA00_000  ├──────────────────────────────┤  <-- segments may not end past here
           │ VGA window (syscall 0x14)    │
0xC00_000  ├──────────────────────────────┤
           │ kernel userland heap, 4 MiB  │  shared by every process
0x1000_000 └──────────────────────────────┘
```

See [Memory Overview](../memory/overview.md) for how the frame is backed physically.

### Pointers passed to syscalls

Every syscall that takes a pointer checks the whole buffer it will touch. The buffer must lie wholly inside the process image (`0x600_000–0xA00_000`) or wholly inside the kernel's user heap (syscall `0x0a`, `0xC00_000–0xFFF_FFF`); otherwise the call returns `InvalidInput` (`0xfc`). Heap memory can be handed to syscalls directly, so a program too large to keep its heap in the image can move it to the user heap (libc++r2's `R2_HEAP_ARENA_KERNEL` and `R2_HEAP_ARENA_GROWING`; [Memento](memento.md) uses the latter), and Go can hand a `KMalloc` block to any syscall through `libgor2.KBytes`. Kernels before this check accepted only the image, and such a program will not run on them.

### Calling convention

| Register | Role |
|----------|------|
| `RAX` | syscall number in, return value out |
| `RDI` | argument 1 |
| `RSI` | argument 2 |
| `R9`  | **clobbered** by every syscall |

The kernel's dispatcher takes exactly two arguments. See the [Syscall Specification](../abi/syscall_specification.md) for details.

### Other constraints

- **8.3 file names.** The shell's `bg`/`fg` take a name of at most 8 characters without the extension. Files on the floppy use FAT 8.3 names.
- **Floating point is not saved across a context switch.** The timer interrupt saves the fifteen general-purpose registers and nothing else (no `fxsave`), so two processes using SSE or x87 at the same time corrupt each other. `gcc -O2` emits SSE too.
- **At most ten processes** exist at once, including the kernel's own tasks (`kmain`, `kclock`, `kshell`, and `init_rc` while it runs).
- **No threads, no signals, no `mmap`, no file descriptors.**

---

## Getting a Program Onto the Machine

There are two places the kernel looks for a binary. `bg <name>` / `fg <name>` search:

1. the shell's working directory (on the FAT12 floppy, or on the ISO when the working directory is under `/mnt/iso`),
2. `/mnt/usb/bin`, then `/mnt/iso/bin`.

So a program can either be copied onto the floppy image:

```
mcopy -o -i fat.img hello.elf ::HELLO.ELF
```

or into `iso/bin/` in the kernel tree before `make build`, which makes it available from any directory. Then, in the kernel shell:

```
fg hello
bg garn --config /mnt/fat/GARN/GARN.CFG
```

Programs started from `INIT.RC` at boot use the same commands (see [Build & Run](../build.md#startup-script-configsinitrc)).

---

## Toolchain

| Tool | Needed for |
|------|------------|
| `gcc` / `g++` (host, x86-64) | C and C++ |
| `nasm` | `_crt0.asm` for C and C++ |
| `llvm-objcopy` | `.bin` flat images (optional) |
| Docker | Go — the TinyGo image is built from `go/tinygo-r2` |
| `mtools` | copying files onto `fat.img` |
| `qemu-system-x86_64` | running |
