#![deny(clippy::indexing_slicing)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points
#![feature(abi_x86_interrupt)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(ptr_internals)]
#![feature(panic_info_message)]

mod macros;
mod multiboot2;

mod acpi;
mod app;
mod fs;
mod init;
mod input;
mod mem;
mod net;
mod sound;
mod time;
mod vga;
//mod video;

use core::panic::PanicInfo;
use core::ptr;

use mem::bump::BumpAllocator;

#[global_allocator]
static mut ALLOCATOR: BumpAllocator = BumpAllocator::new();

//#[lang = "eh_personality"]
//extern "C" fn eh_personality() {}

#[unsafe(no_mangle)]
pub extern "C" fn rust_begin_unwind(_: &core::panic::PanicInfo) {
    //loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() { 
    // VGA buffer position
    let vga_index: &mut isize = &mut 0;

    init_heap_allocator();
    vga::screen::clear(vga_index);

    unsafe {
        init::init(vga_index, init::config::multiboot_ptr);
    }

    // Run prompt loop.
    input::keyboard::keyboard_loop(vga_index);
}

//
//
//

#[lang = "eh_personality"] extern fn eh_personality() {}

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

fn init_heap_allocator() {
    unsafe {
        unsafe extern "C" {
            static __heap_start: u8;
            static __heap_end: u8;
        }

        let heap_start = &__heap_start as *const u8 as usize;
        let heap_end = &__heap_end as *const u8 as usize;
        let heap_size = heap_end - heap_start;

        //#![allow(static_mut_refs)]
        let allocator_ptr = ptr::addr_of_mut!(ALLOCATOR);
        (*allocator_ptr).init(heap_start, heap_size);
    }
}

extern "C" {
    static __stack_start: u8;
    static __stack_end: u8;
}

fn print_stack_info() {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp);
        //string(vga_index, b"Stack pointer: ", Color::Yellow);
        //number(vga_index, sp as u64);
        //newline(vga_index);
    }
}

