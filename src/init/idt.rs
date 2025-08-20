use x86_64::registers::debug;

use crate::abi::idt::{install_isrs, load_idt};

pub fn get_result() -> super::result::InitResult {
    debugln!("Installing Exception handlers and ISRs");
    install_isrs();

    debugln!("Reloading IDT");
    load_idt();

    debugln!("Initializing TSS");
    init_tss();

    debugln!("Resetting TSS");
    let base_addr = unsafe { &tss64 as *const Tss64 as u64 };
    setup_tss_descriptor(base_addr as u64, 0x67);

    debugln!("Reloading GDT");
    reload_gdt();
    
    debugln!("Loading TSS");
    load_tss(0x28);

    debugln!("Done");
    super::result::InitResult::Passed
}

extern "C" {
    static mut tss64: Tss64;
    static mut gdt_start: u8;
    static mut gdt_end: u8;
    static mut gdt_tss_descriptor: [u8; 16];
}

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

fn reload_gdt() {
    unsafe {
        // Prepare GDTR
        let gdtr = DescriptorTablePointer {
            limit: (&gdt_end as *const _ as usize - &gdt_start as *const _ as usize - 1) as u16,
            base: &gdt_start as *const _ as u64,
        };

        // Load new GDT
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &gdtr,
            options(nostack, preserves_flags),
        );
    }
}

fn load_tss(tss_selector: u16) {
    unsafe {
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) tss_selector,
            options(nostack, preserves_flags),
        );
    }
}

fn setup_tss_descriptor(base: u64, limit: u32) {
    let desc = make_tss_descriptor(base, limit);
    unsafe {
        gdt_tss_descriptor.copy_from_slice(&desc);
    }
}

fn make_tss_descriptor(base: u64, limit: u32) -> [u8; 16] {
    let mut desc = [0u8; 16];

    // Limit low 16 bits
    desc[0] = (limit & 0xFF) as u8;
    desc[1] = ((limit >> 8) & 0xFF) as u8;

    // Base low 16 bits
    desc[2] = (base & 0xFF) as u8;
    desc[3] = ((base >> 8) & 0xFF) as u8;

    // Base middle 8 bits
    desc[4] = ((base >> 16) & 0xFF) as u8;

    // Access byte: present, privilege level 0, system segment, type 0x9 (available 64-bit TSS)
    desc[5] = 0b10001001; // P=1, DPL=00, S=0, Type=1001b = 0x9

    // Flags: limit high 4 bits + granularity + 64-bit flag + others
    desc[6] = ((limit >> 16) & 0xF) as u8;

    // Base high 8 bits
    desc[7] = ((base >> 24) & 0xFF) as u8;

    // Base upper 32 bits (for 64-bit base address)
    desc[8] = ((base >> 32) & 0xFF) as u8;
    desc[9] = ((base >> 40) & 0xFF) as u8;
    desc[10] = ((base >> 48) & 0xFF) as u8;
    desc[11] = ((base >> 56) & 0xFF) as u8;

    // The rest (12-15) must be zero per spec
    desc[12] = 0;
    desc[13] = 0;
    desc[14] = 0;
    desc[15] = 0;

    desc
}

#[repr(C, packed)]
struct Tss64 {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved2: u64,
    reserved3: u16,
    io_map_base: u16,
}

fn init_tss() {
    unsafe {
        // Zero out the whole TSS first (probably redundant if in .bss)
        core::ptr::write_bytes(&mut tss64 as *mut _ as *mut u8, 0, core::mem::size_of::<Tss64>());

        // Set kernel stack (top) pointer for ring 0 (rsp0)
        tss64.rsp0 = 0x190000;

        // IST pointers (interrupt stacks)
        // tss64.ist1 = some_stack_address;

        // IO Map base: set to size of TSS to disable IO bitmap
        tss64.io_map_base = core::mem::size_of::<Tss64>() as u16;
    }
}
