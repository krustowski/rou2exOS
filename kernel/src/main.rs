#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;

//mod multiboot2_header;

mod acpi;
mod app;
mod init;
mod input;
mod net;
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
fn panic(info: &PanicInfo) -> ! {
    let vga_index: &mut isize = &mut 0;

    vga::screen::clear(vga_index);

    if let Some(location) = info.location() {
        print_string(vga_index, location.file());
        print_string(vga_index, ":");
        print_num(vga_index, location.line());
        vga::write::newline(vga_index);
    } else {
        vga::write::string(vga_index, b"No location", vga::buffer::Color::Red);
        vga::write::newline(vga_index);
    }

    loop {}
}

fn print_string(vga_index: &mut isize, s: &str) {
    for byte in s.bytes() {
        vga::write::string(vga_index, &[byte], vga::buffer::Color::Red);
    }
}

fn print_num(vga_index: &mut isize, mut num: u32) {
    let mut buf = [0u8; 10]; // Max u32 = 10 digits
    let mut i = buf.len();

    if num == 0 {
        vga::write::string(vga_index, b"0", vga::buffer::Color::Red);
        return;
    }

    while num > 0 {
        i -= 1;
        buf[i] = b'0' + (num % 10) as u8;
        num /= 10;
    }

    for b in &buf[i..] {
        vga::write::string(vga_index, &[*b], vga::buffer::Color::Red);
    }
}
