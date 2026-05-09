# Overview

## Physical Memory Model

The kernel runs at ring 0 with a largely identity-mapped address space (virtual == physical for most addresses). The boot-time page tables are set up by the assembly stage in `boot.asm`; Rust code then adjusts them as needed during `init`.

The Multiboot2 memory map tag (type 6) is parsed at boot and reports usable RAM regions. The kernel does not maintain a physical frame allocator — allocations are handled entirely through pre-reserved static regions described in the linker script and in the page-table pool.

---

## Virtual Address Space Layout

All addresses are 64-bit (x86-64) but the kernel only uses the lower 4 GiB. The page table is a 4-level (PML4) structure; P2 entries use 2 MiB huge pages everywhere except the VGA VRAM window (which uses 4 KiB P1 entries).

| Virtual address   ||        Size   |   Description |
|-------------------||--------------|---------------|
| `0x000_000` | `0x0FF_FFF` |     1 MiB  |   Real-mode legacy (not used at runtime) |
| `0x100_000` | ~            |   varies  |  Kernel image: `.text` `.rodata` `.data` `.bss`; placed by linker at `0x100_000` |
|  `__stack_bottom` | `__stack_top` |  64 KiB |  Boot stack (inside kernel image) |
|  `__heap_start`  | `__heap_end`   | 64 KiB |  Kernel linked-list heap (legacy) |
| `p4_table` / `p3_fb_table` ||    8 KiB  |  Static page tables in `.data` |
| `0x400_000` | `0x5FF_FFF`  |    2 MiB  |   (unused / reserved) |
| `0x600_000` | `0x7FF_FFF`  |    2 MiB  |   ELF userland load region. Each slot's private 2 MiB physical frame is identity-mapped here by `create_user_page_table` |
| `0x800_000` | `0x8FF_FFF` |     1 MiB  |   User stacks (10 slots × 128 KiB spacing) |
| `0x900_000` | `0x9FF_FFF` |     1 MiB  |   (unused userland headroom) |
| `0xA00_000` | `0xAFF_FFF` |    64 KiB  |   VGA graphics RAM window (mapped on demand by syscall `0x14` / `map_vram`). |
| `0xB00_000` | `0xBFF_FFF` |      1 MiB  |   (unmapped; sits between VGA and heap) |
| `0xC00_000` | `0xFFF_FFF` |      4 MiB   |  Userland heap (shared, uheap) |
| `0x1000_000` | `0x1FFF_FFF+` |   varies |   Per-process ELF physical frames: slot 0 → |0x1000_000, slot 1 → 0x1200_000, ... |
| `PAGE_TABLE_POOL` (`.bss`) ||    512 KiB | Static pool for dynamically allocated P4/P3/P2/P1 tables |

---

## Paging Model

### P4 / P3 / P2 structure

The kernel uses a single-level P3 (P4[0] → P3[0] → P2). P2 entries are 2 MiB huge pages (bit 7 set):

```
P4[0] → P3       (kernel P3, shared by all processes)
  P3[0] → P2     (kernel P2, cloned per process)
    P2[0]  → 0x000_000  (2 MiB, kernel image + legacy)
    P2[1]  → 0x200_000  (2 MiB)
    P2[2]  → 0x400_000  (2 MiB)
    P2[3]  → per-slot phys frame  (overridden in user tables)
    P2[4]  → 0x800_000  (2 MiB, user stacks)
    P2[5]  → VGA P1 table  (64 KiB fine-grained, mapped on demand)
    P2[6]  → 0xC00_000  (2 MiB, userland heap, USER+WRITE)
    P2[7]  → 0xE00_000  (2 MiB, userland heap, USER+WRITE)
    ...
```

### Kernel page table vs. user page table

At `init_processes`, `save_kernel_cr3()` snapshots the current CR3 as `KERNEL_CR3`. This is the reference from which all per-process tables are cloned.

`create_user_page_table(slot)` allocates new P4/P3/P2 tables from `PAGE_TABLE_POOL`, copies all 512 entries from the kernel tables, then overrides P2[3] to point at the slot's private 2 MiB physical frame (`0x1000_000 + slot * 0x200_000`). The result is a process that sees:

- its own ELF code/data at virtual `0x600_000` (private frame)
- all kernel mappings everywhere else (shared read-only-ish)
- the shared userland heap at `0xC00_000–0xFFF_FFF` (U/S + R/W, inherited from kernel P2[6/7])

### TLB management

CR3 is written on every context switch in the scheduler. Writing CR3 always flushes the entire TLB (Translation Lookaside Buffer), so no explicit `invlpg` is needed. The `flush_tlb()` helper reloads CR3 with its own current value for cases where only the active process's mappings changed (e.g. after `map_vram`).

---

## Page Table Pool

`PAGE_TABLE_POOL` is a 512 KiB zero-initialised static array in kernel `.bss`, giving a maximum of 128 × 4 KiB pages. `alloc_page()` serves requests from two sources in order:

1. **Free list** (`FREE_LIST` / `FREE_COUNT`) — a fixed-size stack of pointers to pages that were previously returned by `free_page()`. Recycled pages are zeroed before reuse so stale page table entries from the previous owner cannot be followed.
2. **Bump allocator** — if the free list is empty, `NEXT_FREE_PAGE` is advanced and a fresh page is carved from the pool. This pointer only ever increases.

`free_page(p)` pushes a pointer back onto `FREE_LIST`. The list is sized to `PAGE_TABLE_MEMORY_SIZE / 4096` (128 entries) so it can never overflow as long as only genuinely allocated pages are freed.

`free_user_page_table(cr3)` is the public reclamation entry point. It walks the P4 → P3 → P2 chain that `create_user_page_table` built and calls `free_page` for each of the three tables. It also checks P2[5] for a fine-grained P1 table that `map_vram` may have installed; if found (indicated by `PAGE_PS` being clear on a present entry), that page is freed too.

`Scheduler::kill()` calls `free_user_page_table(proc.cr3)` before marking the process `Dead`, then zeros `proc.cr3` to prevent a second call from walking freed memory. Kernel processes (`cr3 == 0`) are skipped automatically.

Each call to `create_user_page_table` consumes 3 pages (P4 + P3 + P2); those pages are returned to the free list when the process exits. `map_vram` consumes 1 additional page per process (but only once — repeated calls reuse the existing P1).

---

## ELF Process Memory Layout (per slot)

```
Physical frame  0x1000_000 + slot*0x200_000   (2 MiB, private)
  ├── ELF PT_LOAD segments written here by load_elf64
  └── zeroed BSS

Virtual 0x600_000 – 0x7FF_FFF   maps to the above private frame
  (P2[3] in the per-process table)

Virtual 0x8x0_000               user stack top (slot-indexed, see table below)
  ├── argv strings and pointers (SysV layout, written by push_user_args)
  └── stack grows downward
```

### User stack tops (by slot)

| Slot | Stack top |
|------|-----------|
| 0 | `0x8F0_000` |
| 1 | `0x8D0_000` |
| 2 | `0x8B0_000` |
| 3 | `0x890_000` |
| 4 | `0x870_000` |
| 5 | `0x850_000` |
| 6 | `0x830_000` |
| 7 | `0x810_000` |
| 8 | `0x7F0_000` |
| 9 | `0x7D0_000` |

Stack slots are assigned by `STACK_NO`, which increments each time `run_elf` is called and wraps modulo 10.

---

## C Library Intrinsics (`c.rs`)

`memcpy`, `memset`, `memmove`, and `memcmp` are provided as `#[no_mangle] extern "C"` functions. They are required by the compiler for struct copies, zero-inits, and `copy_nonoverlapping` fallbacks in `no_std` + `no_libc` builds. All are byte-loop implementations with no SIMD.
