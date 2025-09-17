use crate::init::multiboot2::{tags, parser};
use crate::init::{cpu, idt, heap,pit, fs};
use crate::video::{vga, sysprint::Result as res};




pub fn init(m2_ptr: *mut usize, m2_magic: u32) {
	vga::init_writer();
	clear_screen!();
	result!("First", res::Unknown);
	result!("Second", res::Passed);
	result!("Third", res::Failed);
	result!("Fourth", res::Skipped);



	//unsafe {
	//parser::parse_multiboot2_info(m2_ptr, m2_magic);
	//}

	
	

	



}
