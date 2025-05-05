#![deny(clippy::indexing_slicing)]
#![deny(clippy::panic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

#[unsafe(no_mangle)]
#[unsafe(link_section = ".multiboot2_header")]
pub static MULTIBOOT2_HEADER: [u32; 8] = [
    0xE85250D6, // magic
    0,          // architecture (0 = i386)
    8 * 4,      // header length in bytes (8 entries * 4 bytes)
    0xFFFFFFFFu32 - (0xE85250D6u32 + (8 * 4)) + 1, // checksum
    0, 0,       // dummy tag (type = 0, size = 0, will be ignored)
    0, 8,       // end tag (type = 0, size = 8)
];

mod acpi;
mod app;
mod init;
mod input;
mod mem;
mod net;
mod sound;
mod time;
mod vga;

//use core::alloc::Layout;
use core::panic::PanicInfo;
use core::ptr;

use mem::bump::BumpAllocator;

#[global_allocator]
static mut ALLOCATOR: BumpAllocator = BumpAllocator::new();

//#[entry]
#[unsafe(no_mangle)]
pub extern "C" fn _start() { 
    let vga_index: &mut isize = &mut 0;

    vga::screen::clear(vga_index);

    // Show color palette.
    init::init(vga_index);

    // Run prompt loop.
    input::keyboard::keyboard_loop(vga_index);

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let vga_index: &mut isize = &mut 0;

    init_heap_allocator();

    vga::screen::clear(vga_index);

    if let Some(location) = info.location() {
        print_string(vga_index, location.file());
        print_string(vga_index, ":");
        print_num(vga_index, location.line());
        vga::write::newline(vga_index);
    } else {
        vga::write::string(vga_index, b"No location", 0xc);
        vga::write::newline(vga_index);
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_begin_unwind(_: &core::panic::PanicInfo) {
    //loop {}
}

/*#![alloc_error_handler]
fn alloc_error_handler(_layout: Layout) {
    //loop {}
}*/

fn print_string(vga_index: &mut isize, s: &str) {
    for b in s.as_bytes() {
        vga::write::string(vga_index, &[*b], 0xc);
    }
}

fn print_num(vga_index: &mut isize, mut num: u32) {
    let mut buf = [0u8; 10]; // Max u32 = 10 digits
    let mut i = buf.len();

    if num == 0 {
        vga::write::string(vga_index, b"0", 0xc);
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
        vga::write::string(vga_index, &[*b], 0xc);
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

