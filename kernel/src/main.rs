#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;

//mod multiboot2_header;

mod acpi;
mod init;
mod input;
mod sound;
mod time;
mod vga;

//#[entry]
#[unsafe(no_mangle)]
pub extern "C" fn _start() { 
    let vga_index: &mut isize = &mut 0;

    vga::screen::clear(vga_index);

    // Show color palette.
    init::init(vga_index);

    // Run prompt loop.
    input::keyboard::keyboard_loop(vga_index);

    loop {}
}

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    //let vga_index: &mut isize = &mut 0;

    loop {}
}

