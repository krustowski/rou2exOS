#[no_mangle]
pub extern "C" fn syscall_handler() {
    let syscall_number: u32;
    unsafe {
        core::arch::asm!(
            "mov {0:e}, eax",
            out(reg) syscall_number,
        );
    }

    match syscall_number {
        1 => {
            crate::print!("Syscall 1 called!\n");
        }
        _ => {
            crate::print!("Unknown syscall: ");
            crate::printn!(syscall_number);
            crate::println!();
        }
    }
}

