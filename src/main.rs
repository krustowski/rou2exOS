// Enable static analysis for clippy
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
#[macro_use]
mod macros;
mod multiboot2;

// Core kernel modules
mod acpi;
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
//mod video;

use core::panic::PanicInfo;
use core::ptr;

/// Kernel entrypoint
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() { 
    debugln!("Kernel loaded");

    // VGA buffer position
    let vga_index: &mut isize = &mut 0;
    vga::screen::clear(vga_index);

    // Run init checks
    unsafe {
        init::init(vga_index, init::config::multiboot_ptr as u64);
    }

    // Run prompt loop
    debugln!("Starting shell...");
    input::keyboard::keyboard_loop(vga_index);
}

//
//
//

#[lang = "eh_personality"] extern fn eh_personality() {}

/// Panic handler for panic fucntion invocations
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
    // Implement or stub if needed, but usually core should provide this.
}


/*#![alloc_error_handler]
  fn alloc_error_handler(_layout: Layout) {
  loop {}
  }*/

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
        if let Some(b) = buf.get_mut(i) {
            *b = b'0' + (num % 10) as u8;
        }
        num /= 10;
    }

    for b in buf.get(i..).unwrap_or(&[]) {
        vga::write::string(vga_index, &[*b], vga::buffer::Color::Red);
    }
}

fn print_stack_info() {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp);

        debug!("Stack pointer: ");
        debugn!(sp as u64);
        debugln!("");
    }
}

