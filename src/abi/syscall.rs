/// This function is the syscall ABI dispatching routine. It is called exclusively from the ISR 
/// for interrupt 0x7f. 
#[no_mangle]
pub extern "C" fn syscall_handler() {
    let (syscall_no, arg1, arg2, ret): (u64, u64, u64, u64);

    unsafe {
        core::arch::asm!(
            "mov {0}, rax",
            "mov {1}, rdi",
            "mov {2}, rsi",
            out(reg) syscall_no,
            out(reg) arg1,
            out(reg) arg2,
        );
    }

    debug!("syscall_handler: called: ");
    debugn!(syscall_no);
    debug!(", arg1: ");
    debugn!(arg1);
    debug!(", arg2: ");
    debugn!(arg2);
    debug!("\n");

    rprint!("syscall_handler: called: ");
    rprintn!(syscall_no);
    rprint!(", arg1: ");
    rprintn!(arg1);
    rprint!(", arg2: ");
    rprintn!(arg2);
    rprint!("\n");

    match syscall_no {
        0x10 => {
            // TODO: Verify the pointer!

            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };

            printb!(slice);
            println!("");

            ret = 0;
        }
        _ => {
            debug!("Unknown syscall: ");
            debugn!(syscall_no);
            debugln!("");

            ret = 0xff;
        }
    }

    unsafe {
        core::arch::asm!(
            "mov rax, {0}",
            in(reg) ret,
        );
    }
}

