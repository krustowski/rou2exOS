/// This function is the syscall dispatching routine. It is called exclusively from the ISR for
/// interrupt 0x7f. 
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
            debug!("Syscall 1 called!\n");
        }
        _ => {
            debug!("Unknown syscall: ");
            debugn!(syscall_number);
            debugln!("");
        }
    }
}

