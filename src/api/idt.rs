#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry64 {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    zero: u32,
}

impl IdtEntry64 {
    pub fn new(handler: u64, selector: u16, ist: u8, type_attr: u8) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            ist,
            type_attr,
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            zero: 0,
        }
    }
}

#[repr(C, packed)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

#[link_section = ".idt"]
static mut IDT: [IdtEntry64; 256] = [IdtEntry64 {
    offset_low: 0,
    selector: 0,
    ist: 0,
    type_attr: 0,
    offset_mid: 0,
    offset_high: 0,
    zero: 0,
}; 256];

pub fn load_idt() {
    unsafe {
    let idt_ptr = IdtPointer {
        limit: (core::mem::size_of_val(&IDT) - 1) as u16,
        base: unsafe { &IDT as *const _ as u64 },
    };

        core::arch::asm!("lidt [{}]", in(reg) &idt_ptr);
    }
}

extern "C" {
    fn int7f_isr(); // defined in assembly
}

pub fn init_int7f() {
    let handler_addr = unsafe { int7f_isr as u64 };

    let entry = IdtEntry64::new(
        handler_addr,
        0x08,             // kernel CS selector
        0,                // no IST
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    unsafe {
        IDT[0x7f] = entry;
    }
}

