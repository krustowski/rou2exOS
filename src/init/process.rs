use crate::input::keyboard::keyboard_loop;
use crate::task::{process::Mode, scheduler};
pub unsafe fn init_processes() {
    setup_processes();
}

unsafe fn setup_processes() {
    scheduler::new_process(
        *b"init            ",
        Mode::Kernel,
        clock_test as *const () as u64,
        0x190_000,
    );
    scheduler::new_process(
        *b"clock           ",
        Mode::Kernel,
        clock_test as *const () as u64,
        0x7a0_000,
    );
    scheduler::new_process(
        *b"shell           ",
        Mode::Kernel,
        keyboard_loop as *const () as u64,
        0x700_000,
    );
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
