// Pre-allocated memory space for page tables (e.g., 64 KiB)
const PAGE_TABLE_MEMORY_SIZE: usize = 128 * 1024;
static mut PAGE_TABLE_MEMORY: [u8; PAGE_TABLE_MEMORY_SIZE] = [0; 128 * 1024];
static mut NEXT_FREE_PAGE: usize = 0x1000;

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
    let addr = &mut PAGE_TABLE_MEMORY[NEXT_FREE_PAGE] as *mut u8;
    NEXT_FREE_PAGE += 0x1000;
    core::ptr::write_bytes(addr, 0, 0x1000);
    addr
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
