#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

#[macro_use]
mod debug;
mod multiboot2;
#[macro_use]
mod video;

// Core kernel modules
mod abi;
mod acpi;
mod app;
mod audio;
mod fs;
mod init;
mod input;
mod mem;
mod net;
mod task;
mod time;
mod tui;
mod vga;

/// Kernel entrypoint
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_multiboot2_magic: u32, multiboot_ptr: u32) { 
    debugln!("Kernel loaded");

    // VGA buffer position (LEGACY)
    clear_screen!();

    // TODO: REmove: Instantiate new VGA Writer
    video::vga::init_writer();

    // Run init checks
    //init::init(multiboot_ptr as *mut usize, multiboot2_magic as u32);

	//commented out for now

    // Run the shell loop
    debugln!("Starting shell...");
    println!("Starting shell...\n");
    input::keyboard::keyboard_loop();
	
}

//
//
//

// #[lang = "eh_personality"] extern fn eh_personality() {}

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
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn slice_end_index_len_fail() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[no_mangle]
pub extern "C" fn core_fmt_write() {
    loop {
        x86_64::instructions::hlt();
    }
}

