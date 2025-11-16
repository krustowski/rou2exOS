
//use crate::debug::dump_debug_log_to_file;

use crate::init::{cpu, idt, heap,pit, fs, color, ascii, boot, parser, video};

use crate::video::{vga};
//Results of init system
use crate::video::sysprint::{Result};


pub fn init(m2_ptr: u64) {
	vga::init_writer();
	clear_screen!();
	//TODO: Completely refactor Multiboot2 parsing
    let framebuffer_tag: boot::FramebufferTag = boot::FramebufferTag{
        ..Default::default()
    };
	result!("Kernel Loaded", Result::Passed);

	result!("Checking 64-bit Long Mode", cpu::check());
	
	result!("Reloading IDT and ISRs", idt::idt_isrs_init());
	
	result!("Initializing heap allocation", heap::pmm_heap_init());

	result!("Reading Multiboot2 Tags", parser::parse_info(m2_ptr, &framebuffer_tag));

	result!("Initializing video", video::print_result(&framebuffer_tag));

	result!("Starting PIC timer", pit::pic_pit_init());

	result!("Checking floppy drive", fs::floppy_check_init());

	color::color_demo();
    ascii::ascii_art();

}
