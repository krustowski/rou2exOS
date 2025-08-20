static mut PHYSICAL_BITMAP: [[u64; 8]; 1024] = [[0; 8]; 1024];

static mut PHYSICAL_BASE: u64 = 0;

// Physical memory manager
//
// + 64 bits encode 256 KiB of memory (64 * 4096 bytes)
// + 512 bits encode 2 MiB of memory (8 * 256 KiB)
// + 512 entries in p1 table * 512 tables in p2 table = 1 GiB of memory

// 0xFFFF_FF80_0000_0000
//
// p4[511]
// ...
//

extern "C" {
    pub static mut p4_table: *mut u64;
}

pub enum PhysicalPageStatus {
    FREE = 0x00,
    USED,
    BAD,
}

const P:   u64 = 1 << 0;
const RW:  u64 = 1 << 1;
const US:  u64 = 1 << 2;
const PWT: u64 = 1 << 3;
const PCD: u64 = 1 << 4;
const PS:  u64 = 1 << 7;
const NX:  u64 = 1u64 << 63;

const ADDR_MASK: u64 = 0x000f_ffff_ffff_f000; // 4-KiB aligned phys addr
const TWO_MIB: u64 = 2 * 1024 * 1024;

#[inline(always)]
fn check_bit(word: u64, bit: u32) -> bool {
    (word & (1u64 << bit)) != 0
}

#[inline(always)]
fn set_bit(word: &mut u64, bit: u32) {
    *word |= 1u64 << bit;
}

#[inline(always)]
fn clear_bit(word: &mut u64, bit: u32) {
    *word &= !(1u64 << bit);
}

//
//
//

extern "C" {
    static __kernel_start: u8;
    static __kernel_end: u8;
}

pub unsafe fn reserve_initial_frames() {
    // Reserved frame
    pmm_mark(0, true);

    let kstart = (&__kernel_start as *const _ as u64) >> 12;
    let kend   = (&__kernel_end   as *const _ as u64) >> 12;
    for f in kstart..=kend {
        pmm_mark(f as u32, true);
    }
}

pub unsafe fn build_physmap_2m(p4_virt: *mut u64, mut phys_limit: u64) {
    // Round down phys_limit to a multiple of 2MiB
    phys_limit &= !(TWO_MIB - 1);

    let mut phys: u64 = 0;
    while phys < phys_limit {
        let virt = PHYSICAL_BASE + phys;

        rprint!("Mapping phys ");
        rprintn!(phys);
        rprint!(" address to virt ");
        rprintn!(virt);
        rprint!(" address\n");

        //map_2m(p4_virt, virt, phys, P | RW);
        
        //
        //
        //

        let p4_idx = pml4_index(virt);
        let p3_idx = pdpt_index(virt);
        let p2_idx = pd_index(virt);

        let l3_frame = match pmm_alloc() {
            Some(p) => p,
            None => {
                rprint!("Out of physical memory for page tables\n");
                0
            }
        };

        let l2_frame = match pmm_alloc() {
            Some(p) => p,
            None => {
                rprint!("Out of physical memory for page tables\n");
                0
            }
        };

        //let l3_virt = phys_to_virt_table(l3_frame);
        let l3_virt = l3_frame as *mut u64;
        //let l2_virt = phys_to_virt_table(l2_frame);
        let l2_virt = l2_frame as *mut u64;

        write64(p4_virt.add(p4_idx), (l3_frame & ADDR_MASK_4K) | P | RW);
        write64(l3_virt.add(p3_idx), (l2_frame & ADDR_MASK_4K) | P | RW);
        write64(l2_virt.add(p2_idx), (phys & ADDR_MASK_2M) | P | RW | PS);

        invlpg(virt as usize);

        //
        //
        //

        phys += TWO_MIB;
    }
}

pub unsafe fn pmm_init() {
    let pml4 = read_cr3_phys() as *mut u64;

    rprint!("CR3 physical addr: ");
    rprintn!(read_cr3_phys());
    rprint!("\n");

    rprint!("pml4 virtual addr: ");
    rprintn!(&p4_table as *const _ as u64);
    rprint!("\n");

    rprint!("Setting physical memory base address\n");
    set_physical_base(0xFFFF_8000_0000_0000);

    rprint!("Enabling recursive mapping\n");
    enable_recursive_mapping(pml4);

    rprint!("Marking reserved physical memory frames as used\n");
    reserve_initial_frames();

    rprint!("Building physmap\n");
    build_physmap_2m(pml4, 8 * 1024 * 1024);

    rprint!("Reloading CR3\n");
    reload_cr3();
}

/// Mark a frame by index (0..262143) as used/free
pub unsafe fn pmm_mark(frame_idx: u32, used: bool) {
    let row = (frame_idx / (8 * 64)) as usize;     
    let off = (frame_idx % (8 * 64)) as u32;    

    let col = (off / 64) as usize;              
    let bit = off % 64;                            

    if used { 
        set_bit(&mut PHYSICAL_BITMAP[row][col], bit);
    } else { 
        clear_bit(&mut PHYSICAL_BITMAP[row][col], bit);
    }
}

pub unsafe fn pmm_alloc() -> Option<u64> {
    for row in 0..1024 {
        for col in 0..8 {
            let mut w = PHYSICAL_BITMAP[row][col];

            if w != u64::MAX {
                for bit in 0..64 {
                    if !check_bit(w, bit) {
                        set_bit(&mut w, bit);
                        PHYSICAL_BITMAP[row][col] = w;

                        let frame_idx = (row as u32) * (8 * 64) + (col as u32) * 64 + bit;

                        return Some((frame_idx as u64) << 12);
                    }
                }
            }
        }
    }
    None
}

pub unsafe fn pmm_free(phys: u64) {
    let frame_idx = (phys >> 12) as u32;
    pmm_mark(frame_idx, false);
}

//
//
//

#[inline(always)]
unsafe fn reload_cr3() {
    let mut cr3: u64;

    core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
    core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
}

#[inline(always)]
unsafe fn read_cr3_phys() -> u64 {
    let (frame, _flags) = x86_64::registers::control::Cr3::read();

    frame.start_address().as_u64()
}

pub unsafe fn enable_recursive_mapping(p4_virt: *mut u64) {
    let p4_phys = read_cr3_phys();

    write64(p4_virt.add(510), (p4_phys & ADDR_MASK) | P | RW);
    reload_cr3();
}

#[inline(always)]
fn pml4_index(va: u64) -> usize { 
    ((va >> 39) & 0x1FF) as usize
}

#[inline(always)]
fn pdpt_index(va: u64) -> usize {
    ((va >> 30) & 0x1FF) as usize
}

#[inline(always)]
fn pd_index(va: u64) -> usize {
    ((va >> 21) & 0x1FF) as usize
}

#[inline(always)]
fn pt_index(va: u64) -> usize {
    ((va >> 12) & 0x1FF) as usize
}

//
//
//

pub unsafe fn set_physical_base(base: u64) { 
    PHYSICAL_BASE = base; 
}

#[inline(always)]
pub unsafe fn phys_to_virt(paddr: u64) -> *mut u8 {
    (PHYSICAL_BASE + paddr) as *mut u8
}

#[inline(always)]
pub unsafe fn virt_to_phys(vaddr: *const u8) -> u64 {
    (vaddr as u64) - PHYSICAL_BASE
}

#[inline(always)]
unsafe fn phys_to_virt_table(phys: u64) -> *mut u64 {
    (PHYSICAL_BASE + (phys & ADDR_MASK)) as *mut u64
}

//
//
//

#[inline(always)]
unsafe fn read64(p: *const u64) -> u64 {
    core::ptr::read_volatile(p)
}

#[inline(always)]
unsafe fn write64(p: *mut u64, val: u64) {
    core::ptr::write_volatile(p, val);
}

#[inline(always)]
unsafe fn invlpg(addr: usize) {
    core::arch::asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
}

//
//
//

//
// VMM
//

unsafe fn ensure_table(parent_tbl: *mut u64, idx: usize, table_flags: u64) -> *mut u64 {
    let e = read64(parent_tbl.add(idx));

    if e & P != 0 {
        return e as *mut u64;
        //return phys_to_virt_table(e);
    }

    let phys = match pmm_alloc() {
        Some(p) => p,
        None => panic!("Out of physical memory for page tables"),
    };

    // Zero the new table (512 * 8 bytes)
    //let tbl = phys_to_virt_table(phys);
    let tbl = phys as *mut u64;
    for i in 0..512 {
        write64(tbl.add(i), 0);
    }
    write64(parent_tbl.add(idx), (phys & ADDR_MASK) | table_flags);
    tbl
}

//
//
//

const ADDR_MASK_4K: u64 = 0x000F_FFFF_FFFF_F000;
const ADDR_MASK_2M: u64 = 0x000F_FFFF_FFE0_0000; // bits 20..0 zero

#[inline(always)]
fn is_aligned_2m(x: u64) -> bool { (x & 0x1F_FFFF) == 0 } // 2 MiB

pub unsafe fn map_2m(p4_virt: *mut u64, virt: u64, phys: u64, pde_flags: u64) {
    // L3 (PDPT)
    rprint!("L3 ensure_table()\n");
    let l3 = ensure_table(p4_virt, pml4_index(virt), P | RW);

    // Clear 1GiB if present
    let l3e_ptr = l3.add(pdpt_index(virt));
    let l3e = read64(l3e_ptr);
    if (l3e & P) != 0 && (l3e & PS) != 0 {
        rprint!("Clearing 1GiB mapping\n");
        write64(l3e_ptr, 0);
        invlpg(virt as usize);
    }

    // L2 (PD)
    rprint!("L2 ensure_table()\n");
    let l2 = ensure_table(l3, pdpt_index(virt), P | RW);

    // Write 2MiB PDE (PS=1)
    let l2e_ptr = l2.add(pd_index(virt));
    let entry = (phys & ADDR_MASK_2M) | pde_flags | PS;
    write64(l2e_ptr, entry);

    invlpg(virt as usize);
}

pub unsafe fn map_4k(p4_virt: *mut u64, virt: u64, phys: u64, pte_flags: u64) {
    let l3 = ensure_table(p4_virt, pml4_index(virt), P | RW);

    let l3e_ptr = l3.add(pdpt_index(virt));
    let l3e = read64(l3e_ptr);
    if l3e & P != 0 && (l3e & PS) != 0 {
        write64(l3e_ptr, 0);
        invlpg(virt as usize);
    }

    let l2 = ensure_table(l3, pdpt_index(virt), P | RW);

    let l2e_ptr = l2.add(pd_index(virt));
    let l2e = read64(l2e_ptr);
    if l2e & P != 0 && (l2e & PS) != 0 {
        write64(l2e_ptr, 0);
        invlpg(virt as usize);
    }

    let l1 = ensure_table(l2, pd_index(virt), P | RW);

    let pte_ptr = l1.add(pt_index(virt));
    let entry = (phys & ADDR_MASK_4K) | pte_flags;
    write64(pte_ptr, entry);

    invlpg(virt as usize);
}

//
//
//

pub unsafe fn map_framebuffer(phys: u64, virt: u64) {
    let p4_virt = read_cr3_phys() as *mut u64;

    let p4_idx = pml4_index(virt);
    let p3_idx = pdpt_index(virt);
    let p2_idx = pd_index(virt);

    let l3_frame = match pmm_alloc() {
        Some(p) => p,
        None => {
            rprint!("Out of physical memory for page tables\n");
            0
        }
    };

    let l2_frame = match pmm_alloc() {
        Some(p) => p,
        None => {
            rprint!("Out of physical memory for page tables\n");
            0
        }
    };

    //let l3_virt = phys_to_virt_table(l3_frame);
    let l3_virt = l3_frame as *mut u64;
    //let l2_virt = phys_to_virt_table(l2_frame);
    let l2_virt = l2_frame as *mut u64;

    write64(p4_virt.add(p4_idx), (l3_frame & ADDR_MASK_4K) | P | RW);
    write64(l3_virt.add(p3_idx), (l2_frame & ADDR_MASK_4K) | P | RW);

    for i in 0..4 {
        write64(l2_virt.add(p2_idx + i), (phys + (i as u64 * 0x200000) & ADDR_MASK_2M) | P | RW | PS);
    }

    invlpg(virt as usize);
}

