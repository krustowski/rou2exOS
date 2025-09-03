use crate::init::multiboot2::{tags, parser};
use crate::init::{cpu, idt, heap,pit, fs};
use crate::video::{vga};
use spin::Mutex;


static const  tempbuff = vga::SysBuffer::new();

pub fn init(m2_ptr: *mut usize, m2_magic: u32) {
	debugln!("Kernel init start");
	//unsafe {
	//parser::parse_multiboot2_info(m2_ptr, m2_magic);
	//}

	
	

	



}
