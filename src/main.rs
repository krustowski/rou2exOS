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
mod audio;
mod fs;
mod init;
mod input;
mod mem;
mod net;
mod task;
mod time;
// TBD
mod vga;

/// Kernel entrypoint
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_multiboot2_magic: u32, multiboot_ptr: u32) {
    debugln!("Kernel loaded");

    // Run init checks and initialize system
    init::check::init(multiboot_ptr);

    unsafe {
        task::scheduler::idle(0xff);

        loop {
            core::arch::asm!("pause");
        }
    }
}

//
//
//

use core::panic::PanicInfo;

/// Panic handler for panic fucntion invocations
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    debugln!("kernel panic!");
    clear_screen!();
    error!("KERNEL PANIC\n");

    if let Some(location) = info.location() {
        warn!(location.file());
        warn!(": ");
        printn!(location.line() as u64);
        println!();
    } else {
        warn!("no location\n")
    }

    unsafe {
        loop {
            core::arch::asm!("hlt");
        }
    }
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
