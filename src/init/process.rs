use crate::input::{cmd, keyboard::keyboard_loop};
use crate::task::{process::Mode, scheduler};
pub unsafe fn init_processes() {
    // Snapshot the boot-time CR3 before any per-process tables are created.
    crate::mem::pages::save_kernel_cr3();
    // Map 0xC00_000–0xFFF_FFF as user-accessible and initialise the heap.
    // Must run after save_kernel_cr3 so create_user_page_table inherits the
    // updated P2[6/7] entries when it clones the kernel page table.
    crate::mem::uheap::init();
    setup_processes();
}

unsafe fn setup_processes() {
    // Slot 0 must be a sentinel for the kernel's boot execution context.
    // The scheduler's current_pid starts at 0; on the very first PIT tick
    // it saves the kernel's idle RSP into slot 0 before switching away.
    // Without this sentinel that write would clobber the iretq entry frame
    // of whichever real process occupies slot 0, preventing it from ever
    // executing its entry function.
    scheduler::new_process(
        *b"kmain           ",
        Mode::Kernel,
        kernel_idle as *const () as u64,
        0,
        0,
    );
    scheduler::new_process(
        *b"init_rc         ",
        Mode::Kernel,
        init_rc as *const () as u64,
        0,
        0,
    );
    scheduler::new_process(
        *b"clock           ",
        Mode::Kernel,
        clock_test as *const () as u64,
        0x2B0_000,
        0,
    );
    let shell_pid = scheduler::new_process(
        *b"shell           ",
        Mode::Kernel,
        keyboard_loop as *const () as u64,
        0x2D0_000,
        0,
    );
    scheduler::set_shell_pid(shell_pid);
}

/// Absorbs the kernel's boot RSP on the first PIT tick. Stays resident as an idle hlt loop.
#[no_mangle]
extern "C" fn kernel_idle() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

/// Reads INIT.RC from FAT12 root and executes each line through the shell dispatcher.
/// After the script finishes the process kills itself — it has no further purpose.
#[no_mangle]
extern "C" fn init_rc() -> ! {
    use crate::fs::fat12::{
        block::Floppy,
        fs::{fat83, Filesystem},
    };

    let mut buf = [0u8; 1024];

    let floppy = Floppy::init();
    if let Ok(fs) = Filesystem::new(&floppy) {
        let name83 = fat83(b"INIT.RC");
        if let Some(entry) = fs.find_entry(0, &name83) {
            fs.read_file(entry.start_cluster, &mut buf);

            let mut start = 0usize;
            for i in 0..buf.len() {
                let ch = buf[i];
                if ch == b'\n' || ch == b'\0' {
                    let mut line = &buf[start..i];
                    // strip trailing CR for files edited on Windows
                    if line.last() == Some(&b'\r') {
                        line = &line[..line.len() - 1];
                    }
                    if !line.is_empty() && line[0] != b'#' {
                        cmd::handle(line);
                    }
                    if ch == b'\0' {
                        break;
                    }
                    start = i + 1;
                }
            }
        }
    }

    let pid = unsafe { scheduler::get_current_pid() };
    unsafe {
        scheduler::kill(pid);
    }
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[no_mangle]
extern "C" fn clock_test() -> ! {
    use crate::vga::buffer::Color;
    use crate::vga::write::{newline, number, string};

    loop {
        unsafe {
            let vga_index: &mut isize = &mut 144;

            let (_, _o, _, h, m, s) = crate::time::rtc::read_rtc_full();

            if h < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, h as u64);
            string(vga_index, b":", crate::vga::buffer::Color::White);
            if m < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, m as u64);
            string(vga_index, b":", Color::White);
            if s < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, s as u64);
            newline(vga_index);

            for _ in 0..50_000 {
                core::arch::asm!("pause");
            }
        }
    }
}
