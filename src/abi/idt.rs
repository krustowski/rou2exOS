use x86_64::{registers::control::Cr2, structures::{gdt::DescriptorFlags, idt::{PageFaultErrorCode}}};

use crate::input::keyboard::keyboard_loop;

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

#[repr(C)]
pub struct InterruptStackFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C, packed)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

#[link_section = ".bss.idt"]
#[no_mangle]
static mut IDT: [IdtEntry64; 256] = [IdtEntry64 {
    offset_low: 0,
    selector: 0,
    ist: 0,
    type_attr: 0,
    offset_mid: 0,
    offset_high: 0,
    zero: 0,
}; 256];

//
//
//

/*static mut IDT: Mutex<InterruptDescriptorTable> = Mutex::new({
  unsafe {
  let mut idt = InterruptDescriptorTable::new();

// Rust exception handlers
idt.page_fault.set_handler_fn(page_fault_handler);
idt.double_fault.set_handler_fn(double_fault_handler);
idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);

// Custom int 0x7f ISR (manually set the handler address)
idt[0x7f].set_handler_addr(VirtAddr::new(int7f_isr as u64))
.set_privilege_level(x86_64::PrivilegeLevel::Ring3) // if needed
.set_code_selector(SegmentSelector::new(0x08, x86_64::PrivilegeLevel::Ring3))
.set_present(true);

idt
}
});*/

//
//
//

extern "C" {
    fn int7f_isr();
    fn syscall_80h();
}


#[no_mangle]
#[link_section = ".text"]
extern "C" fn page_fault_handler(stack_frame: u64, error_code: u64) {
    error!("EXCEPTION: PAGE FAULT");
    warn!("\nAccessed Address: ");

    match Cr2::read() {
        Ok(addr) => {
            printn!(addr.as_u64());
        }
        Err(e) => {
            warn!("Cannot read CR2");
        }
    }

    warn!("\nError Code: ");
    printn!(error_code);
    warn!("\nStack frame: ");
    printn!(stack_frame);
    print!("\n\n");

    keyboard_loop();
}

#[no_mangle]
#[link_section = ".text"]
extern "C" fn general_protection_fault_handler(error_code: u64) {
    unsafe {
        error!("EXCEPTION: GENERAL PROTECTION FAULT");

        warn!("\nError code: ");
        printn!(error_code);
        print!("\n\n");

        keyboard_loop();
    }
}

#[no_mangle]
#[link_section = ".text"]
extern "C" fn invalid_opcode_handler(stack_frame: *mut InterruptStackFrame) {
    unsafe {
        error!("EXCEPTION: INVALID OPCODE");

        let frame = &mut *stack_frame;

        warn!("\nRIP: ");
        printn!(frame.rip);
        warn!("\nStack frame: ");
        printn!(frame.rsp);
        print!("\n\n");

        // Try to skip the invalid opcode 
        //frame.rip += 3;
        keyboard_loop();
    }
}

#[no_mangle]
#[link_section = ".text"]
extern "C" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    error!("EXCEPTION: DOUBLE FAULT");

    warn!("\nError Code: ");
    printn!(error_code);
    warn!("\nStack frame: ");
    printn!(stack_frame.rsp);
    print!("\n\nRecovering the shell...", crate::video::vga::Color::White);
    print!("\n\n");

    // Recover the shell
    keyboard_loop();
}

//
//
//

pub fn load_idt() {
    unsafe {
        let idt_ptr = IdtPointer {
            limit: (core::mem::size_of_val(&IDT) - 1) as u16,
            base: &IDT as *const _ as u64,
        };

        core::arch::asm!("lidt [{}]", in(reg) &idt_ptr, options(nostack, preserves_flags));
    }
}

pub fn install_isrs() {
    let entry_06 = IdtEntry64::new(
        invalid_opcode_handler as u64,
        0x08,             
        0,                
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    let entry_08 = IdtEntry64::new(
        double_fault_handler as u64,
        0x08,             
        0,                
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    let entry_0d = IdtEntry64::new(
        general_protection_fault_handler as u64,
        0x08,             
        0,                
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    let entry_0e = IdtEntry64::new(
        page_fault_handler as u64,
        0x08,             
        0,                
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    let entry_7f = IdtEntry64::new(
        int7f_isr as u64,
        0x1b,             
        0,                
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    let entry_80 = IdtEntry64::new(
        syscall_80h as u64,
        0x1b,             
        0,                
        0b1110_1110,      // present, DPL=3, 64-bit interrupt gate
    );

    unsafe {
        IDT[0x06] = entry_06;
        IDT[0x08] = entry_08;
        IDT[0x0d] = entry_0d;
        IDT[0x0e] = entry_0e;
        IDT[0x7f] = entry_7f;
        IDT[0x80] = entry_80;
    }
}

