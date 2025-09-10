use crate::init::multiboot2::{tags, parser};
use crate::init::{cpu, idt, heap,pit, fs};
use crate::video::{vga};
use crate::video::macros::{system};
use spin::Mutex;


//static tempbuff = vga::SysBuffer::new();

pub fn init(m2_ptr: *mut usize, m2_magic: u32) {
	debugln!("Kernel init start");
	vga::init_writer();
	result!("Test");

	//unsafe {
	//parser::parse_multiboot2_info(m2_ptr, m2_magic);
	//}

	
	

	



}
