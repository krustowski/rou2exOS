use crate::input::keyboard::keyboard_loop;
use crate::task::{process::Mode, scheduler};
pub unsafe fn init_processes() {
    // Snapshot the boot-time CR3 before any per-process tables are created.
    crate::mem::pages::save_kernel_cr3();
    setup_processes();
}

unsafe fn setup_processes() {
    // Kernel process stacks must not overlap the userland virtual region
    // 0x600_000–0x7FF_FFF that per-process page tables remap per-slot.
    // 0x2B0_000 and 0x2D0_000 are in P2[1] (0x200_000–0x3FF_FFF), which is
    // identity-mapped with kernel-only flags and always safe for ring-0 stacks.
    scheduler::new_process(
        *b"init            ",
        Mode::Kernel,
        clock_test as *const () as u64,
        0x190_000,
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

#[no_mangle]
extern "C" fn clock_test() -> ! {
    use crate::vga::buffer::Color;
    use crate::vga::write::{newline, number, string};

    loop {
        unsafe {
            let vga_index: &mut isize = &mut 144;

            let (_, _o, _, h, m, s) = crate::time::rtc::read_rtc_full();

            // Hours
            if h < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, h as u64);
            string(vga_index, b":", crate::vga::buffer::Color::White);

            // Minutes
            if m < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, m as u64);
            string(vga_index, b":", Color::White);

            // Seconds
            if s < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, s as u64);
            newline(vga_index);

            for _ in 0..50_000 {
                //core::arch::asm!("mov rdx, 0", "int 0x7f", "hlt");
                core::arch::asm!("pause");
            }
        }
    }
}
