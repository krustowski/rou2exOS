#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::{PanicInfo};

mod init;
mod input;
mod sound;
mod time;
mod vga;

//pub static mut vga_index: &mut isize = &mut 0;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    let vga_index: &mut isize = &mut 0;

        vga::screen::clear(vga_index);

        // Show color palette.
        init::init(vga_index);

        // Run prompt loop.
        input::keyboard::keyboard_loop(vga_index);
}

