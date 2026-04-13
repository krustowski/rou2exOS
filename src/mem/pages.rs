extern "C" {
    static p4_table: u64;
}

unsafe fn walk_pml4() {
    let p4: [u64; 512];
    let mut p3: [u64; 512];
    let mut p2: [u64; 512];
    let mut p1: [u64; 512];

    for i in 0..512 {
        for j in 0..512 {
            for k in 0..512 {}
        }
    }
}

//
//
//

pub const PAGE_SIZE: usize = 4096;
pub const ENTRIES: usize = 512;

// Page flags
pub const PRESENT: u64 = 1 << 0;
pub const WRITE: u64 = 1 << 1;
pub const USER: u64 = 1 << 2;
pub const NX: u64 = 1 << 63;

// We'll keep a very simple frame allocator
static mut NEXT_FREE_FRAME: usize = 0x400000; // after kernel

pub static mut KERNEL_HH_MAPPED: bool = false;
pub static mut KERNEL_PML4: u64 = 0;

pub unsafe fn alloc_frame() -> usize {
    let frame = NEXT_FREE_FRAME;
    NEXT_FREE_FRAME += PAGE_SIZE;
    // clear the page for safety
    core::ptr::write_bytes(frame as *mut u8, 0, PAGE_SIZE);
    frame
}

pub unsafe fn map_page(pml4_phys: usize, virt: usize, phys: usize, flags: u64) {
    let pml4 = pml4_phys as *mut u64;

    let pml4_idx = (virt >> 39) & 0x1FF;
    let pdpt_idx = (virt >> 30) & 0x1FF;
    let pd_idx = (virt >> 21) & 0x1FF;
    let pt_idx = (virt >> 12) & 0x1FF;

    // ---------------- PML4 ----------------
    let pml4_entry = pml4.add(pml4_idx);
    let pdpt_ptr: *mut u64;

    if (*pml4_entry & PRESENT) == 0 {
        let frame = alloc_frame();
        *pml4_entry = (frame as u64) | PRESENT | WRITE | (flags & USER);
        pdpt_ptr = frame as *mut u64;
    } else {
        if (flags & USER) != 0 {
            *pml4_entry |= USER; // propagate USER upward
        }
        pdpt_ptr = (*pml4_entry & 0x000F_FFFF_FFFF_F000) as *mut u64;
    }

    // ---------------- PDPT ----------------
    let pdpt_entry = pdpt_ptr.add(pdpt_idx);
    let pd_ptr: *mut u64;

    if (*pdpt_entry & PRESENT) == 0 {
        let frame = alloc_frame();
        *pdpt_entry = (frame as u64) | PRESENT | WRITE | (flags & USER);
        pd_ptr = frame as *mut u64;
    } else {
        if (flags & USER) != 0 {
            *pdpt_entry |= USER;
        }
        pd_ptr = (*pdpt_entry & 0x000F_FFFF_FFFF_F000) as *mut u64;
    }

    // ---------------- PD ----------------
    let pd_entry = pd_ptr.add(pd_idx);
    let pt_ptr: *mut u64;

    if (*pd_entry & PRESENT) == 0 {
        let frame = alloc_frame();
        *pd_entry = (frame as u64) | PRESENT | WRITE | (flags & USER);
        pt_ptr = frame as *mut u64;
    } else {
        if (flags & USER) != 0 {
            *pd_entry |= USER;
        }
        pt_ptr = (*pd_entry & 0x000F_FFFF_FFFF_F000) as *mut u64;
    }

    // ---------------- PT (final page) ----------------
    let pt_entry = pt_ptr.add(pt_idx);

    let phys_clean = (phys as u64) & 0x000F_FFFF_FFFF_F000;

    *pt_entry = phys_clean | flags | PRESENT;
}

pub unsafe fn copy_kernel_half(old_pml4: usize, new_pml4: usize) {
    let old = old_pml4 as *mut u64;
    let new = new_pml4 as *mut u64;

    for i in 0..2 {
        new.add(i).write(old.add(i).read());
    }

    for i in 256..512 {
        new.add(i).write(old.add(i).read());
    }
}

pub unsafe fn new_process_pml4(user_base: usize, user_size: usize) -> usize {
    let new_pml4 = alloc_frame();
    copy_kernel_half(KERNEL_PML4 as usize, new_pml4);

    // Map identity-mapped user pages
    let mut addr = user_base;
    while addr < user_base + user_size {
        map_page(new_pml4, addr, addr, PRESENT | WRITE | USER);
        addr += PAGE_SIZE;
    }

    new_pml4
}
