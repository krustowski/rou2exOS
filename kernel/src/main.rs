#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;

mod vga;
mod init;
mod input;

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
    let mut vga_index = 0;

    vga::screen::clear(&mut vga_index);

    // Show color palette.
    init::init(&mut vga_index);

    // Run prompt loop.
    input::keyboard::keyboard_loop(&mut vga_index);
}

