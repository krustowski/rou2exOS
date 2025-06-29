// Enable static analysis features for clippy
#![deny(clippy::indexing_slicing)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(ptr_internals)]
#![feature(panic_info_message)]

#[macro_use]
mod debug;
mod multiboot2;
#[macro_use]
mod video;

// Core kernel modules
mod acpi;
mod api;
mod app;
mod audio;
mod fs;
mod init;
mod input;
mod mem;
mod net;
mod time;
mod tui;
mod vga;

/// Kernel entrypoint
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() { 
    debugln!("Kernel loaded");

    // VGA buffer position (LEGACY)
    clear_screen!();

    // TODO: REmove: Instantiate new VGA Writer
    video::vga::init_writer();

    // Run init checks
    unsafe {
        init::init(init::config::multiboot_ptr as u64);
    }

    // Run the shell loop
    debugln!("Starting shell...");
    input::keyboard::keyboard_loop();
}

//
//
//

#[lang = "eh_personality"] extern fn eh_personality() {}

use core::panic::PanicInfo;

/// Panic handler for panic fucntion invocations
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use vga::write::{string, number, newline};
    use vga::buffer::Color;

    let vga_index: &mut isize = &mut 0;

    vga::screen::clear(vga_index);

    if let Some(location) = info.location() {
        string(vga_index, location.file().as_bytes(), Color::Red);
        string(vga_index, b":", Color::Red);
        number(vga_index, location.line() as u64);
        newline(vga_index);
    } else {
        string(vga_index, b"No location", Color::Red);
        newline(vga_index);
    }

    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_begin_unwind(_: &core::panic::PanicInfo) {
    //loop {}
}

#[no_mangle]
pub extern "C" fn panic_bounds_check() -> ! {
    //panic("bounds check failed");
    loop {}
}

#[no_mangle]
pub extern "C" fn slice_end_index_len_fail() -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn core_fmt_write() {
    loop {}
}

