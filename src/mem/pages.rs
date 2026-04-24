const PAGE_TABLE_MEMORY_SIZE: usize = 256 * 1024;

#[repr(C, align(4096))]
struct PageTablePool {
    data: [u8; PAGE_TABLE_MEMORY_SIZE],
}

static mut PAGE_TABLE_POOL: PageTablePool = PageTablePool {
    data: [0; PAGE_TABLE_MEMORY_SIZE],
};
static mut NEXT_FREE_PAGE: usize = 0;

/// Physical address of the kernel's boot-time P4 table.  Saved once during
/// early init so the scheduler can restore it when switching to a kernel process.
pub static mut KERNEL_CR3: u64 = 0;

/// Capture the current CR3 as the kernel's master page table reference.
/// Must be called before any userland processes are spawned.
pub unsafe fn save_kernel_cr3() {
    KERNEL_CR3 = read_cr3() as u64;
}

/// Returns the physical base address of the 2 MiB frame reserved for userland
/// slot `slot`.  Slots map to distinct non-overlapping frames starting at 16 MiB.
pub fn user_frame_phys(slot: usize) -> u64 {
    0x1000_000 + slot as u64 * 0x200_000
}

/// Build a per-process P4/P3/P2 hierarchy for userland slot `slot`.
///
/// The new tables share the kernel's identity-mapped entries for all addresses
/// outside 0x600_000–0x7FF_FFF.  P2[3] is overridden to point at the slot's
/// private 2 MiB physical frame so that every userland process sees its own
/// code/data at virtual address 0x600_000 without interference.
///
/// Returns the physical address of the new P4 (suitable for writing to CR3).
pub unsafe fn create_user_page_table(slot: usize) -> u64 {
    let kernel_p4 = KERNEL_CR3 as *mut u64;
    let kernel_p3 = (*kernel_p4 & 0x000f_ffff_ffff_f000) as *mut u64;
    let kernel_p2 = (*kernel_p3 & 0x000f_ffff_ffff_f000) as *mut u64;

    let new_p4 = alloc_page() as *mut u64;
    let new_p3 = alloc_page() as *mut u64;
    let new_p2 = alloc_page() as *mut u64;

    // Clone kernel tables so all existing mappings (framebuffer, high memory,
    // etc.) remain accessible from userland processes.
    core::ptr::copy_nonoverlapping(kernel_p4, new_p4, 512);
    core::ptr::copy_nonoverlapping(kernel_p3, new_p3, 512);
    core::ptr::copy_nonoverlapping(kernel_p2, new_p2, 512);

    // Give the slot its own private 2 MiB frame at vaddr 0x600_000.
    let phys_frame = user_frame_phys(slot);
    *new_p2.add(3) = phys_frame | PAGE_PRESENT | PAGE_WRITE | PAGE_USER | PAGE_PS;

    // Wire P3[0] → new_p2, P4[0] → new_p3.
    *new_p3 = new_p2 as u64 | PAGE_PRESENT | PAGE_WRITE | PAGE_USER;
    *new_p4 = new_p3 as u64 | PAGE_PRESENT | PAGE_WRITE | PAGE_USER;

    new_p4 as u64
}

/// Read the current CR3 value (physical address of the active PML4).
#[inline]
pub unsafe fn read_cr3() -> *mut u64 {
    let cr3: u64;
    core::arch::asm!(
        "mov {}, cr3",
        out(reg) cr3,
        options(nostack, preserves_flags),
    );
    cr3 as *mut u64
}

/// Flush the TLB by reloading CR3.
#[inline]
unsafe fn flush_tlb() {
    let cr3: u64;
    core::arch::asm!(
        "mov {0}, cr3",
        "mov cr3, {0}",
        out(reg) cr3,
        options(nostack, preserves_flags),
    );
}

const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITE: u64 = 1 << 1;
const PAGE_USER: u64 = 1 << 2;
const PAGE_PS: u64 = 1 << 7; // huge page (2 MiB at P2 level)

/// Walk the active 4-level page table and ensure every existing mapping that
/// covers [virt_start, virt_end) has the User (U/S) bit set at all levels.
///
/// The kernel uses 2 MiB huge pages at the P2 level, so this only touches P4,
/// P3, and P2 entries — no P1 tables to allocate.  Any entry that is not
/// already present is left alone (the mapping must have been created first).
pub unsafe fn ensure_user_pages(virt_start: u64, virt_end: u64) {
    let p4 = read_cr3();

    // Align start down and end up to 2 MiB boundaries.
    let start_2m = virt_start & !0x1F_FFFF;
    let end_2m = (virt_end + 0x1F_FFFF) & !0x1F_FFFF;

    let mut addr = start_2m;
    while addr < end_2m {
        let p4_idx = ((addr >> 39) & 0x1FF) as usize;
        let p3_idx = ((addr >> 30) & 0x1FF) as usize;
        let p2_idx = ((addr >> 21) & 0x1FF) as usize;

        // ── P4 ──────────────────────────────────────────────────────────────
        let p4e = p4.add(p4_idx);
        if *p4e & PAGE_PRESENT == 0 {
            addr += 0x4000_0000; // skip entire 1 GiB
            continue;
        }
        *p4e |= PAGE_USER | PAGE_WRITE;

        let p3 = (*p4e & 0x000f_ffff_ffff_f000) as *mut u64;

        // ── P3 ──────────────────────────────────────────────────────────────
        let p3e = p3.add(p3_idx);
        if *p3e & PAGE_PRESENT == 0 {
            addr += 0x4000_0000; // skip 1 GiB block
            continue;
        }
        // If P3 entry is itself a 1 GiB huge page, just set User and move on.
        if *p3e & PAGE_PS != 0 {
            *p3e |= PAGE_USER | PAGE_WRITE;
            addr += 0x4000_0000;
            continue;
        }
        *p3e |= PAGE_USER | PAGE_WRITE;

        let p2 = (*p3e & 0x000f_ffff_ffff_f000) as *mut u64;

        // ── P2 ──────────────────────────────────────────────────────────────
        let p2e = p2.add(p2_idx);
        if *p2e & PAGE_PRESENT != 0 {
            *p2e |= PAGE_USER | PAGE_WRITE;
        }

        addr += 0x20_0000; // next 2 MiB
    }

    flush_tlb();
}

unsafe fn alloc_page() -> *mut u8 {
    if NEXT_FREE_PAGE + 0x1000 > PAGE_TABLE_MEMORY_SIZE {
        panic!("Out of preallocated page table memory!");
    }
    let addr = &mut PAGE_TABLE_POOL.data[NEXT_FREE_PAGE] as *mut u8;
    NEXT_FREE_PAGE += 0x1000;
    core::ptr::write_bytes(addr, 0, 0x1000);
    addr
}

/// Map physical VGA graphics RAM (0xA0000–0xAFFFF, 64 KiB) into the current
/// process's page table at virtual 0xA00_000, with USER+WRITE access.
/// Safe to call multiple times; only installs a fresh P1 if P2[5] has no P1
/// pointer yet (huge-page entries are replaced).  Returns 0xA00_000 on
/// success, 0 if the page-table walk fails.
pub unsafe fn map_vram() -> u64 {
    const VRAM_PHYS: u64 = 0xA_0000; // physical: 640 KiB (EGA/VGA window)
    const VRAM_VIRT: u64 = 0xA00_000; // virtual:  10 MiB (dedicated slot)
    const P2_IDX: usize = 5; // (0xA00_000 >> 21) & 0x1FF

    let p4 = read_cr3();
    let p4e = *p4;
    if p4e & PAGE_PRESENT == 0 {
        return 0;
    }
    let p3 = (p4e & 0x000f_ffff_ffff_f000) as *mut u64;

    let p3e = *p3;
    if p3e & PAGE_PRESENT == 0 {
        return 0;
    }
    let p2 = (p3e & 0x000f_ffff_ffff_f000) as *mut u64;

    let existing = *p2.add(P2_IDX);
    // Already a fine-grained P1 pointer (not a huge page): mapping is intact.
    if existing & PAGE_PRESENT != 0 && existing & PAGE_PS == 0 {
        return VRAM_VIRT;
    }

    // Allocate a P1 table and map 16 × 4 KiB pages → physical VRAM.
    let p1 = alloc_page() as *mut u64;
    for i in 0usize..16 {
        *p1.add(i) = (VRAM_PHYS + i as u64 * 0x1000) | PAGE_PRESENT | PAGE_WRITE | PAGE_USER;
    }
    *p2.add(P2_IDX) = p1 as u64 | PAGE_PRESENT | PAGE_WRITE | PAGE_USER;
    flush_tlb();

    VRAM_VIRT
}

pub unsafe fn map_32mb(p4: *mut u64, phys_start: usize, virt_start: usize) {
    let p4_index = (virt_start >> 39) & 0x1FF;
    let p3_index = (virt_start >> 30) & 0x1FF;
    // let p2_index = (virt_start >> 21) & 0x1FF;
    // let p1_index = (virt_start >> 12) & 0x1FF;

    let p3 = get_or_alloc_table(p4.add(p4_index).as_mut().unwrap());
    //let p3 = alloc_page();
    p4.add(p4_index).write(p3 as u64 | 0x3); // present + writable

    let p2 = get_or_alloc_table(p3.add(p3_index).as_mut().unwrap());
    //let p2 = alloc_page();
    p3.add(p3_index).write(p2 as u64 | 0x3);

    for i in 0..16 {
        let p1 = alloc_page();
        p2.add(i).write(p1 as u64 | 0x3);

        for j in 0..512 {
            let frame = phys_start + ((i * 512 + j) * 0x1000);
            p1.add(j).write(frame as u8 | 0x3);
        }
    }
}

pub unsafe fn identity_map(p4: *mut u64, size: usize) {
    let page_count = size / 0x1000;
    let p1_tables = page_count.div_ceil(512);

    let p3 = get_or_alloc_table(p4);
    p4.write(p3 as u64 | 0x3);

    let p2 = get_or_alloc_table(p3);
    p3.write(p2 as u64 | 0x3);

    for i in 0..p1_tables {
        let p1 = alloc_page();
        debug!("identity_map: allocating P1[");
        debugn!(i);
        debug!("] = ");
        debugn!(p1);
        debugln!("");
        p2.add(i).write(p1 as u64 | 0x3);
        for j in 0..512 {
            let page_idx = i * 512 + j;
            if page_idx >= page_count {
                break;
            }
            let phys = (page_idx * 0x1000) as u64;
            p1.add(j).write(phys as u8 | 0x3);
        }
    }
}

unsafe fn get_or_alloc_table(entry: *mut u64) -> *mut u64 {
    let val = entry.read();
    if val & 1 == 0 {
        let new_page = alloc_page() as u64 | 0x3;
        entry.write(new_page);
        (new_page & 0x000f_ffff_ffff_f000) as *mut u64
    } else {
        (val & 0x000f_ffff_ffff_f000) as *mut u64
    }
}
