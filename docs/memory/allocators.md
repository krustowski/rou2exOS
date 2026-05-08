# Allocators

There are three distinct allocators in the kernel, each serving a different purpose and lifetime.

---

## 1. Kernel Linked-List Heap (`mem/heap.rs`)

**Region:** `__heap_start – __heap_end` (64 KiB, defined in `linker.ld`, lives in the kernel binary after `.bss`)

**Purpose:** General kernel-internal allocations during early init (tested in `init/heap.rs`). This is the original, hand-written heap; it is present but seldom used now that most kernel subsystems use static arrays.

### Block Layout

Each allocation is preceded by an inline `HeapNode` header (4 pointer-sized fields, ~32 bytes on x86-64):

```
struct HeapNode {
    size:     usize,           // data bytes available after this header
    status:   HeapNodeStatus,  // Free | Used | Unknown
    previous: *mut HeapNode,   // linked-list predecessor (null for first)
    next:     *mut HeapNode,   // linked-list successor (null for last)
}
```

At `init()`, a single free node spanning the entire heap is written at `__heap_start`. `HEAP_PTR` tracks the "current" node for scanning.

### Allocation (`alloc`)

1. Walk the list from `HEAP_PTR` looking for a free node large enough for `alloc_size`.
2. If the found node is significantly larger, `split()` it: the left node gets exactly `alloc_size` bytes (minimum `MIN_HEAP_NODE_SIZE = 16`), the right node gets the remainder. Both are marked Free; the caller immediately marks the left one Used.
3. Zero the data region of the allocated node.
4. Return the address of the `HeapNode` header (callers treat this as the usable address — note: the returned pointer is the header itself, not the data after it).

### Deallocation (`free`)

1. Mark the node at `vaddr` as Free.
2. Call `merge()`, which walks backwards until it finds a free node and merges it with its free successor by summing sizes and updating the next/prev pointers.

### Limitations

- The allocator has bugs in its `merge` / `alloc` walk logic (it can loop indefinitely and has an `OOM` fallback counter). It should be considered legacy.
- No thread safety — no lock is held during allocation.
- Minimum heap node size is `MIN_HEAP_NODE_SIZE = 16` bytes; smaller requests are rounded up.
- Total heap size: 64 KiB, fixed at compile time.

---

## 2. Kernel Rust Global Allocator — Bump Allocator (`mem/bump.rs`)

**Region:** Configured at runtime via `BumpAllocator::init(heap_start, heap_size)`.

**Purpose:** Satisfies Rust's `#[global_allocator]` trait so that `Box`, `Vec`, and other `alloc` crate types can be used inside the kernel. Currently initialised by `init_heap_allocator()` in `init/heap.rs`, which points it at the same `__heap_start / __heap_end` region.

### Algorithm

```
next  ← heap_start  (AtomicUsize, SeqCst)

alloc(layout):
    aligned = (next + align - 1) & !(align - 1)
    new_next = aligned + size
    if new_next > heap_end: return null
    next.store(new_next)
    return aligned

dealloc(_ptr, _layout):
    /* no-op */
```

Allocation is a single atomic fetch-and-add (effectively). Deallocation does nothing — freed memory is not reclaimed.

### Properties

| Property | Value |
|----------|-------|
| Thread safety | Yes (SeqCst atomic) |
| Deallocation | No (bump; memory is permanent) |
| Fragmentation | None (monotone) |
| Max allocation | Heap size − alignment padding |

The bump allocator is appropriate for the kernel because most kernel-level Rust allocations are long-lived static structures. Any allocation that the kernel makes via `Box::new` or similar is effectively leaked for the kernel's lifetime.

---

## 3. Userland Heap — Free-List Allocator (`mem/uheap.rs`)

**Region:** `0xC00_000 – 0xFFF_FFF` (4 MiB, virtual == physical, identity-mapped)

**Purpose:** Dynamic heap for userland processes. Exposed as syscalls `0x0a` (malloc), `0x0b` (realloc), `0x0f` (free). All processes share one physical heap — there is no per-process isolation.

**Mapping:** `uheap::init()` is called from `init_processes()` after `save_kernel_cr3()`. It sets P2[6] (`0xC00_000`) and P2[7] (`0xE00_000`) to USER+WRITE 2 MiB huge pages in the current page table. Because `create_user_page_table` clones the kernel P2, every subsequent user process inherits these entries automatically.

### Block Layout (in-band, 8-byte header)

```
 ┌─────────────────┬─────────────────┐
 │   data_size: u32 │     flags: u32  │  ← 8-byte header
 └─────────────────┴─────────────────┘
 │           data region              │  ← data_size bytes
 └────────────────────────────────────┘
```

`flags` bit 0: `1` = free, `0` = used. No other bits are used.

At `init()`, a single free block spanning `HEAP_SIZE − 8` bytes is written at `HEAP_START`.

### Allocation (`malloc`)

1. Acquire `LOCK` (spin mutex).
2. Linear scan from `HEAP_START`: skip blocks where `flags & FLAG_FREE == 0` or `data_size < size`.
3. On finding a suitable free block:
   - If `remainder = data_size − size ≥ HDR + MIN_SPLIT (16)`: split — write a new free header at `addr + HDR + size`, mark the found block used with `data_size = size`.
   - Otherwise: use the whole block (no split), mark used.
4. Zero the data region.
5. Return `addr + HDR` (pointer to data, not header).

All sizes are rounded up to 8-byte alignment (`align8`) before the scan.

### Reallocation (`realloc`)

Tries three strategies in order, under the lock:

1. **Shrink / same size**: If `new_size ≤ old_size`, optionally split the tail into a new free block if the remainder is large enough. Return the same pointer.
2. **In-place expansion**: If the immediately adjacent next block is free and `old_size + HDR + next_size ≥ new_size`, absorb it. Split the remainder if large enough. Return the same pointer.
3. **Allocate + copy + free**: `alloc_inner(new_size)`, `copy_nonoverlapping`, `set_free(old_hdr)`, `coalesce()`. Returns the new pointer (old pointer is invalid after this).

Special cases: `ptr == 0` → `malloc(new_size)`; `new_size == 0` → `free(ptr)`.

### Deallocation (`free`)

1. Validate `ptr ∈ [HEAP_START + HDR, HEAP_END)`.
2. Acquire `LOCK`.
3. `set_free(ptr − HDR)`: write `FLAG_FREE` to the header flags field.
4. `coalesce()`: single linear pass that merges any pair of adjacent free blocks by summing `[size + HDR + next_size]` into the left block's header.

### Locking

`static LOCK: Mutex<()>` (spin). All public functions (`malloc`, `realloc`, `free`) acquire the lock before calling the inner `unsafe` helpers. The lock is released when the guard drops at the end of the function. Because the kernel re-enables interrupts (`sti`) at the top of `syscall_inner`, a PIT tick can preempt a syscall; `SCHEDULER.try_lock()` in the scheduler handles this by skipping the tick.

### Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `HEAP_START` | `0xC00_000` | First byte of the heap region |
| `HEAP_END` | `0x1000_000` | One past the last byte (exclusive) |
| `HDR` | `8` | Header size in bytes |
| `FLAG_FREE` | `1` | Free flag in the flags field |
| `MIN_SPLIT` | `16` | Minimum data size for a remainder block to be split off |

### Pointer Validation in Syscalls

Syscall handlers validate userland pointers against `USERLAND_START (0x600_000) ≤ ptr ≤ USERLAND_END (0xA00_000)`. Heap pointers (`0xC00_000–0xFFF_FFF`) fall **outside** this range and are therefore rejected by syscalls that check pointer arguments (e.g. `0x10 print`, `0x13 write_vga`). Userland code must copy data from heap memory into its statically-allocated buffers before passing addresses to such syscalls.

The `malloc`/`realloc`/`free` syscalls themselves (`0x0a`, `0x0b`, `0x0f`) do not check pointers against the userland range — `uheap::free` validates against `HEAP_START/HEAP_END` instead.
