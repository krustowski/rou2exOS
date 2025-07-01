// Pre-allocated memory space for page tables (e.g., 64 KiB)
static mut PAGE_TABLE_MEMORY: [u8; 128 * 1024] = [0; 128 * 1024];
static mut NEXT_FREE_PAGE: usize = 0x1000;

unsafe fn alloc_page() -> *mut u8 {
    if NEXT_FREE_PAGE + 0x1000 > PAGE_TABLE_MEMORY.len() {
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
    let p2_index = (virt_start >> 21) & 0x1FF;
    let p1_index = (virt_start >> 12) & 0x1FF;

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
    let p1_tables = (page_count + 511) / 512;

    let p4_index = (0x0000_0000_0000_0000u64 >> 39) & 0x1FF;
    let p3_index = (0x0000_0000_0000_0000u64 >> 30) & 0x1FF;
    let p2_index = (0x0000_0000_0000_0000u64 >> 21) & 0x1FF;

    let p3 = get_or_alloc_table(p4.add(p4_index as usize));
    p4.add(p4_index as usize).write(p3 as u64 | 0x3);

    let p2 = get_or_alloc_table(p3.add(p3_index as usize));
    p3.add(p3_index as usize).write(p2 as u64 | 0x3);

    for i in 0..p1_tables {
        let p1 = alloc_page();
        debug!("identity_map: allocating P1[");
        debugn!(i);
        debug!("] = ");
        debugn!(p1);
        debugln!("");
        p2.add(p2_index as usize + i).write(p1 as u64 | 0x3);
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
        return (new_page & 0x000f_ffff_ffff_f000) as *mut u64;
    } else {
        return (val & 0x000f_ffff_ffff_f000) as *mut u64;
    }
}

