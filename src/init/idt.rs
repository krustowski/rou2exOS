use crate::abi::idt::{install_isrs, load_idt};

pub fn get_result() -> super::result::InitResult {
    debugln!("Installing Exception handlers and ISRs");
    install_isrs();

    debugln!("Reloading IDT");
    load_idt();

    debugln!("Resetting TSS");
    setup_tss_descriptor(0x90_000, 0x67);

    debugln!("Reloading GDT");
    reload_gdt();
    
    debugln!("Loading TSS");
    load_tss(0x28);

    super::result::InitResult::Passed
}

extern "C" {
    static mut tss64: [u8; 104];
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

    // For TSS, the G (granularity) bit and L (64-bit) bit are zero, so just limit bits

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

