/// Userland heap allocator — 0xC00_000 to 0xFFF_FFF (4 MiB, P2[6–7]).
///
/// Memory is identity-mapped (physical == virtual) so the kernel can
/// manipulate it directly while userland reads/writes allocated blocks
/// through the same virtual addresses.  All processes share one heap;
/// the spin lock serialises concurrent syscall-level access.
///
/// Block layout (in-band header, 8 bytes):
///   [u32 data_size][u32 flags]   flags bit 0: 1=free, 0=used
/// Minimum data region per block: MIN_SPLIT bytes (prevents infinite splitting).
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

pub const HEAP_START: u64 = 0xC00_000;
pub const HEAP_END: u64 = 0x1000_000; // exclusive (4 MiB region)
const HEAP_SIZE: usize = (HEAP_END - HEAP_START) as usize;

const HDR: u64 = 8; // header size in bytes
const FLAG_FREE: u32 = 1;
const MIN_SPLIT: usize = 16; // min data size to bother splitting a block

static LOCK: Mutex<()> = Mutex::new(());
static READY: AtomicBool = AtomicBool::new(false);

// Page-table flags (copied locally to avoid importing pages.rs internals)
const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITE: u64 = 1 << 1;
const PAGE_USER: u64 = 1 << 2;
const PAGE_PS: u64 = 1 << 7; // 2 MiB huge page at P2 level

/// Call once during kernel init (after `save_kernel_cr3`).
/// Maps P2[6] and P2[7] as USER+WRITE 2 MiB huge pages, then writes
/// the initial free block covering the whole 4 MiB heap.
pub fn init() {
    unsafe {
        map_heap_pages();
        write_hdr(HEAP_START, (HEAP_SIZE - HDR as usize) as u32, FLAG_FREE);
    }
    READY.store(true, Ordering::Release);
    rprint!("uheap: 4 MiB userland heap at 0xC00_000\n");
}

/// Allocate `size` bytes.  Returns the data-region virtual address (inside
/// [HEAP_START+8, HEAP_END)), or 0 on failure.  Data is zeroed.
pub fn malloc(size: usize) -> u64 {
    if !READY.load(Ordering::Acquire) || size == 0 {
        return 0;
    }
    let size = align8(size);
    let _g = LOCK.lock();
    unsafe { alloc_inner(size) }
}

/// Reallocate `ptr` to `new_size` bytes.
/// `ptr == 0` behaves like `malloc(new_size)`.
/// `new_size == 0` frees `ptr` and returns 0.
pub fn realloc(ptr: u64, new_size: usize) -> u64 {
    if !READY.load(Ordering::Acquire) {
        return 0;
    }
    if ptr == 0 {
        return malloc(new_size);
    }
    if new_size == 0 {
        free(ptr);
        return 0;
    }
    let new_size = align8(new_size);
    let _g = LOCK.lock();
    unsafe { realloc_inner(ptr, new_size) }
}

/// Free a block previously returned by `malloc` or `realloc`.
pub fn free(ptr: u64) {
    if !READY.load(Ordering::Acquire) {
        return;
    }
    if ptr < HEAP_START + HDR || ptr >= HEAP_END {
        return;
    }
    let _g = LOCK.lock();
    unsafe {
        set_free(ptr - HDR);
        coalesce();
    }
}

// ── inner (lock already held) ────────────────────────────────────────────────

unsafe fn alloc_inner(size: usize) -> u64 {
    let mut addr = HEAP_START;
    while addr + HDR <= HEAP_END {
        let (blk, flags) = read_hdr(addr);
        if flags & FLAG_FREE != 0 && blk as usize >= size {
            let remainder = blk as usize - size;
            if remainder >= HDR as usize + MIN_SPLIT {
                // Split: carve `size` bytes off the front, leave remainder free.
                let next = addr + HDR + size as u64;
                write_hdr(next, (remainder - HDR as usize) as u32, FLAG_FREE);
                write_hdr(addr, size as u32, 0);
            } else {
                // Use the whole block (no split).
                write_hdr(addr, blk, 0);
            }
            let data = addr + HDR;
            core::ptr::write_bytes(data as *mut u8, 0, size);
            return data;
        }
        if blk == 0 {
            break; // corrupted or end sentinel
        }
        addr += HDR + blk as u64;
    }
    0 // out of memory
}

unsafe fn realloc_inner(ptr: u64, new_size: usize) -> u64 {
    let hdr_addr = ptr - HDR;
    let (old_size, _) = read_hdr(hdr_addr);
    let old_size = old_size as usize;

    // Shrink or same size — optionally split off the tail.
    if new_size <= old_size {
        let remainder = old_size - new_size;
        if remainder >= HDR as usize + MIN_SPLIT {
            write_hdr(hdr_addr, new_size as u32, 0);
            write_hdr(ptr + new_size as u64, (remainder - HDR as usize) as u32, FLAG_FREE);
            coalesce();
        }
        return ptr;
    }

    // Try in-place expansion: absorb the immediately adjacent free block.
    let next_addr = ptr + old_size as u64;
    if next_addr + HDR <= HEAP_END {
        let (next_size, next_flags) = read_hdr(next_addr);
        if next_flags & FLAG_FREE != 0 {
            let combined = old_size + HDR as usize + next_size as usize;
            if combined >= new_size {
                let remainder = combined - new_size;
                if remainder >= HDR as usize + MIN_SPLIT {
                    write_hdr(hdr_addr, new_size as u32, 0);
                    write_hdr(ptr + new_size as u64, (remainder - HDR as usize) as u32, FLAG_FREE);
                } else {
                    write_hdr(hdr_addr, combined as u32, 0);
                }
                return ptr;
            }
        }
    }

    // Fall back: allocate a new block, copy data, free the old block.
    let new_ptr = alloc_inner(new_size);
    if new_ptr == 0 {
        return 0;
    }
    core::ptr::copy_nonoverlapping(ptr as *const u8, new_ptr as *mut u8, old_size);
    set_free(hdr_addr);
    coalesce();
    new_ptr
}

/// Linear-scan coalescing: merge pairs of adjacent free blocks.
unsafe fn coalesce() {
    let mut addr = HEAP_START;
    while addr + HDR <= HEAP_END {
        let (size, flags) = read_hdr(addr);
        if size == 0 {
            break;
        }
        let next = addr + HDR + size as u64;
        if flags & FLAG_FREE != 0 && next + HDR <= HEAP_END {
            let (next_size, next_flags) = read_hdr(next);
            if next_flags & FLAG_FREE != 0 {
                let merged = size as usize + HDR as usize + next_size as usize;
                write_hdr(addr, merged as u32, FLAG_FREE);
                continue; // re-examine from the same addr
            }
        }
        addr += HDR + size as u64;
    }
}

// ── page-table helpers ───────────────────────────────────────────────────────

unsafe fn map_heap_pages() {
    let p4 = crate::mem::pages::read_cr3();
    if (*p4) & PAGE_PRESENT == 0 {
        return;
    }
    let p3 = ((*p4) & 0x000f_ffff_ffff_f000) as *mut u64;
    if (*p3) & PAGE_PRESENT == 0 {
        return;
    }
    let p2 = ((*p3) & 0x000f_ffff_ffff_f000) as *mut u64;

    let huge = PAGE_PRESENT | PAGE_WRITE | PAGE_USER | PAGE_PS;

    // P2[6] → physical 0xC00_000 (virtual 0xC00_000–0xDFF_FFF)
    let e6 = p2.add(6);
    if (*e6) & PAGE_PRESENT == 0 {
        *e6 = 0xC00_000 | huge;
    } else {
        *e6 |= PAGE_USER | PAGE_WRITE;
    }

    // P2[7] → physical 0xE00_000 (virtual 0xE00_000–0xFFF_FFF)
    let e7 = p2.add(7);
    if (*e7) & PAGE_PRESENT == 0 {
        *e7 = 0xE00_000 | huge;
    } else {
        *e7 |= PAGE_USER | PAGE_WRITE;
    }

    // TLB flush (reload CR3 with itself)
    let _cr3: u64;
    core::arch::asm!(
        "mov {0}, cr3",
        "mov cr3, {0}",
        out(reg) _cr3,
        options(nostack, preserves_flags),
    );
}

// ── header I/O ───────────────────────────────────────────────────────────────

#[inline(always)]
unsafe fn read_hdr(addr: u64) -> (u32, u32) {
    let p = addr as *const u32;
    (p.read_volatile(), p.add(1).read_volatile())
}

#[inline(always)]
unsafe fn write_hdr(addr: u64, size: u32, flags: u32) {
    let p = addr as *mut u32;
    p.write_volatile(size);
    p.add(1).write_volatile(flags);
}

#[inline(always)]
unsafe fn set_free(hdr_addr: u64) {
    let p = (hdr_addr as *mut u32).add(1);
    p.write_volatile(FLAG_FREE);
}

#[inline(always)]
fn align8(n: usize) -> usize {
    (n + 7) & !7
}
