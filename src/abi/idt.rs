use x86_64::{registers::control::Cr2, structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}};

use crate::{abi::syscall::{syscall_80h, syscall_handler}, input::keyboard::keyboard_loop};

#[link_section = ".data.idt"]
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

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

#[no_mangle]
#[link_section = ".text"]
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    error!("EXCEPTION: PAGE FAULT");
    warn!("\nAccessed Address: ");

    match Cr2::read() {
        Ok(addr) => {
            printn!(addr.as_u64());
        }
        Err(_) => {
            warn!("Cannot read CR2");
        }
    }

    warn!("\nError Code: ");
    printn!(error_code.bits());
    warn!("\nStack frame: ");
    printn!(stack_frame.stack_pointer.as_u64());
    print!("\n\n");

    keyboard_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(_frame: InterruptStackFrame, error_code: u64) {
    error!("EXCEPTION: GENERAL PROTECTION FAULT");

    warn!("\nError code: ");
    printn!(error_code);
    print!("\n\n");

    keyboard_loop();
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: INVALID OPCODE");

    warn!("\nRIP: ");
    printn!(stack_frame.instruction_pointer.as_u64());
    warn!("\nStack frame: ");
    printn!(stack_frame.stack_pointer.as_u64());
    print!("\n\n");

    // Try to skip the invalid opcode 
    //frame.rip += 3;
    keyboard_loop();
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    error!("EXCEPTION: DOUBLE FAULT");

    warn!("\nError Code: ");
    printn!(error_code);
    warn!("\nStack frame: ");
    printn!(stack_frame.stack_pointer.as_u64());
    print!("\n\nRecovering the shell...", crate::video::vga::Color::White);
    print!("\n\n");

    // Recover the shell
    keyboard_loop();
}

pub fn load_idt() {
    #[expect(static_mut_refs)]
    unsafe { IDT.load() };
}

#[no_mangle]
extern "x86-interrupt" fn timer_handler(_stack: InterruptStackFrame) {
    // Acknowledge the PIC
    crate::input::port::write(0x20, 0x20);

    crate::task::schedule();  // Switch tasks
}

extern "x86-interrupt" fn keyboard_handler(_stack: InterruptStackFrame) {
    let scancode = crate::input::port::read_u8(0x60);

    unsafe {
        #[expect(static_mut_refs)] // this is bad but i cant figure out how to fix
        for s in crate::input::irq::RECEPTORS.iter() {
            if s.pid != 0 {
                s.push_irq(scancode);
            }
        }
    }

    // Acknowledge the PIC
    crate::input::port::write(0x20, 0x20);
}

extern "x86-interrupt" fn floppy_drive_handler(_stack: InterruptStackFrame) {
    // Acknowledge the PIC
    //crate::input::port::write(0x20, 0x20);
}

#[expect(static_mut_refs)]
/// https://phrack.org/issues/59/4
pub fn install_isrs() {
    unsafe { IDT.invalid_opcode.set_handler_fn(invalid_opcode_handler) };
    unsafe { IDT.double_fault.set_handler_fn(double_fault_handler) };
    unsafe { IDT.general_protection_fault.set_handler_fn(general_protection_fault_handler) };
    unsafe { IDT.page_fault.set_handler_fn(page_fault_handler) };

    unsafe { IDT[0x20].set_handler_fn(timer_handler) };
    unsafe { IDT[0x21].set_handler_fn(keyboard_handler) };
    unsafe { IDT[0x26].set_handler_fn(floppy_drive_handler) };
    unsafe { IDT[0x7f].set_handler_fn(syscall_handler).set_privilege_level(x86_64::PrivilegeLevel::Ring3) };
    unsafe { IDT[0x80].set_handler_fn(syscall_80h) };
}
