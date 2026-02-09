use core::arch::naked_asm;

use x86_64::{
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
    VirtAddr,
};

use crate::{
    abi::syscall::{syscall_80h, syscall_handler},
    input::keyboard::keyboard_loop,
    task::process::{crash, idle, resume},
};

#[link_section = ".idt"]
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

#[no_mangle]
#[link_section = ".text"]
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    error!("EXCEPTION: PAGE FAULT");
    warn!("\nAccessed Address: ");

    rprint!("EXCEPTION: PAGE FAULT\n");

    match Cr2::read() {
        Ok(addr) => {
            printx!(addr.as_u64());
        }
        Err(_) => {
            warn!("Cannot read CR2");
        }
    }

    warn!("\nError Code: ");
    printn!(error_code.bits());
    warn!("\nStack frame SP: ");
    printx!(stack_frame.stack_pointer.as_u64());
    print!("\n\n");

    //keyboard_loop();
    unsafe {
        resume(2);
        crash();
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    frame: InterruptStackFrame,
    error_code: u64,
) {
    error!("EXCEPTION: GENERAL PROTECTION FAULT");

    warn!("\nError code: ");
    printn!(error_code);
    warn!("\nStack frame SP: ");
    printx!(frame.stack_pointer.as_u64());
    print!("\n\n");

    //keyboard_loop();
    unsafe {
        resume(2);
        crash();
    }
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: INVALID OPCODE");

    warn!("\nRIP: ");
    printx!(stack_frame.instruction_pointer.as_u64());
    warn!("\nStack frame SP: ");
    printx!(stack_frame.stack_pointer.as_u64());
    print!("\n\n");

    //keyboard_loop();
    unsafe {
        resume(2);
        crash();
    }
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    error!("EXCEPTION: DOUBLE FAULT");

    warn!("\nError Code: ");
    printn!(error_code);
    warn!("\nStack frame: ");
    printx!(stack_frame.stack_pointer.as_u64());
    /*print!(
        "\n\nRecovering the shell...",
        crate::video::vga::Color::White
    );*/
    print!("\n\n");

    //keyboard_loop();
    unsafe {
        resume(2);
        crash();

        loop {
            core::arch::asm!("hlt");
        }
    }
}

pub fn load_idt() {
    #[expect(static_mut_refs)]
    unsafe {
        IDT.load()
    };
}

extern "C" {
    fn timer_interrupt_stub();
}

/*#[no_mangle]
extern "x86-interrupt" fn timer_handler(stack: InterruptStackFrame) {
    unsafe {
        let context = &crate::task::process::Context {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rdi: 0,
            rsi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            //rip: stack.instruction_pointer.as_u64(),
            rip: 0,
            cs: stack.code_segment.0 as u64,
            rflags: stack.cpu_flags.bits(),
            rsp: stack.stack_pointer.as_u64(),
            ss: stack.stack_segment.0 as u64,
        };

        crate::task::process::schedule(context);
        crate::input::port::write(0x20, 0x20);
    }
}*/

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
    unsafe {
        IDT.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(1)
    };
    unsafe {
        IDT.general_protection_fault
            .set_handler_fn(general_protection_fault_handler)
    };
    unsafe {
        IDT.page_fault
            .set_handler_fn(page_fault_handler)
            .set_stack_index(2)
    };

    //unsafe { IDT[0x20].set_handler_fn(timer_handler) };
    unsafe { IDT[0x20].set_handler_addr(VirtAddr::new(timer_interrupt_stub as u64)) };
    unsafe { IDT[0x21].set_handler_fn(keyboard_handler) };
    unsafe { IDT[0x26].set_handler_fn(floppy_drive_handler) };
    unsafe {
        IDT[0x7f]
            .set_handler_fn(syscall_handler)
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3)
    };
    unsafe { IDT[0x80].set_handler_fn(syscall_80h) };
}
