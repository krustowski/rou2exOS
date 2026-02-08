//use crate::debug::dump_debug_log_to_file;

use core::result;

use crate::init::{ascii, boot, color, cpu, fs, heap, idt, parser, pit, video};

use crate::task::task::INIT_DONE;
use crate::video::vga;
//Results of init system
use crate::video::sysprint::Result;

pub static mut FRAMEBUFFER_PTR: boot::FramebufferTag = boot::FramebufferTag {
    addr: 0x0,
    bpp: 0,
    fb_type: 0,
    typ: 0,
    pitch: 0,
    reserved: 0,
    height: 0,
    width: 0,
    size: 0,
};

pub fn init(m2_ptr: u32) {
    vga::init_writer();
    clear_screen!();

    rprint!("Init start!\n");

    //TODO: Completely refactor Multiboot2 parsing
    /*let framebuffer_tag: boot::FramebufferTag = boot::FramebufferTag {
        ..Default::default()
    };*/
    result!("Kernel Loaded", Result::Passed);

    result!("Enabling SSE and syscalls", cpu::check());

    rprint!("SSE Enabled!\n");

    result!("Reloading GDT, TSS, IDT and ISRs", idt::idt_isrs_init());

    rprint!("Tables reloaded!\n");

    result!("Reading Multiboot2 tags", unsafe {
        parser::parse_info(m2_ptr, &mut FRAMEBUFFER_PTR)
    });

    rprint!("Multiboot2 tags parsed!\n");

    result!("Initializing heap allocation", heap::pmm_heap_init());

    result!("Initializing video", unsafe {
        video::print_result(&FRAMEBUFFER_PTR)
    });

    result!("Starting PIC timer", pit::pic_pit_init());

    result!("Checking floppy drive", fs::floppy_check_init());

    color::color_demo();
    ascii::ascii_art();

    unsafe {
        crate::task::process::setup_processes();
        INIT_DONE = true;
    }
}
