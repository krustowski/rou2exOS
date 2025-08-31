use super::{m2_header,tags};

/*use crate::{debug::dump_debug_log_to_file, init::{config::{p1_fb_table, p1_fb_table_2, p2_fb_table, p3_fb_table, p4_table}, font::{draw_text_psf, parse_psf}}, mem, vga::{
    buffer::Color, write::{newline, number, string}
} };
use super::{result::InitResult}; */

/*pub fn print_info(multiboot_ptr: u64, mut fb_tag: &FramebufferTag) -> InitResult {
    unsafe {
        debug!("Multiboot2 pointer: ");
        debugn!(multiboot_ptr);
        debugln!("");

        if parse_multiboot2_info((multiboot_ptr as u64) as usize, fb_tag) > 0 {
            return InitResult::Passed;
        }
    }

    debug!("Multiboot2 pointer: ");
    debugn!(multiboot_ptr);
    debugln!("");

    InitResult::Failed
}
*/


/*     let addr = align_up(base_addr, 8);

    // First 4 bytes: total size of the multiboot info
    let total_size = *(addr as *const u32) as usize;

    let mut ptr = addr + 8;
    let end = addr + total_size;

    let mut tag_count = 0;

*/




//static mut U_MEM: UsableMemory = UsableMemory{start: 0, end: 0, count: 0}; //change this accordingly!!! placeholder for now


pub unsafe fn parse_multiboot2_info(m2_ptr: *mut usize, m2_magic: u32) {
	//check magic, needs to match 
	if m2_magic != m2_header::MULTIBOOT2_BOOTLOADER_MAGIC {
		return; //return 1 here???
	};
	//alignment to 8
	let mut m2_tag = &*((m2_ptr as *mut u8).add(8) as *mut tags::TagHeader);


    while m2_tag.typ != m2_header::M2TagType::End {

        match m2_tag.typ {

            m2_header::M2TagType::CmdLine => {
				break;
            }

            m2_header::M2TagType::Module => { 
                break;
            }

            m2_header::M2TagType::Mmap => {
                let mmap_tag = &*(m2_tag as *const _ as *const tags::MemoryMapTag);
				memory_map_tag(mmap_tag); 
            }

            m2_header::M2TagType::Framebuffer => {
				break;

            }

            m2_header::M2TagType::AcpiOLD => {
				break;
            }

            _ => {
				break;
            }
        }

        m2_tag = &*((align_up(m2_tag.size as usize, 8) as *mut tags::TagHeader));


    }


}

pub unsafe fn memory_map_tag(mut mmap_tag: &tags::MemoryMapTag) {

	let tag_size = mmap_tag.size as usize;
	let entry_size = mmap_tag.entry_size as usize; //incrementation

	let mut entries_start = mmap_tag as *const _ as *mut u8;
	let mut tag_end = entries_start.add(tag_size);
 





}

//stashed code for now!!!


pub unsafe fn acpi_old_tag() {
	/* 
	                debugln!("ACPI v1 Root System Descriptor Pointer tag");

                let acpi_tag = &*(ptr as *const AcpiRSDPTag);
                debug!("Signature: ");
                debug!(acpi_tag.signature);
                debug!("\nOEM: ");
                debug!(acpi_tag.oemid);
                debugln!("");

                let acpi_sdt = &*(acpi_tag.rsdt_addr as *const AcpiSDTHeader);
	*/
}

pub unsafe fn module_tag() {
	debug!("Module tag found: ");
	/* 
    //let start = *((ptr + 8) as *const u32);
    //let end = *((ptr + 12) as *const u32);
    let str_ptr = ptr + 16;
    let str_len = tag.size as usize - 16;
    let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

    let cmdline = core::str::from_utf8_unchecked(raw_bytes);
    debugln!(cmdline);
	*/
}



pub unsafe fn boot_line_tag() {
	debug!("Boot command line tag: ");

    /*let str_ptr = ptr + 8;
    let str_len = tag.size as usize - 8;
    let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

    let cmdline = core::str::from_utf8_unchecked(raw_bytes);
    debugln!(cmdline);
	*/
}

pub unsafe fn framebuffer_tag() {
	debugln!("Framebuffer tag: ");
	/* 
    b_tag = &*(ptr as *const FramebufferTag);

    debug!("Framebuffer address: ");
    debugn!(fb_tag.addr as u64);
    debugln!("");

    debug!("(bpp + res): ");
    debugn!(fb_tag.bpp as u64);
    debug!(" + ");
    debugn!(fb_tag.width as u64);
    debug!("x");
    debugn!(fb_tag.height as u64);
    debugln!("");

    debug!("Pitch: ");
    debugn!(fb_tag.pitch);
    debugln!("");


                use core::ptr;
                use x86_64::registers::control::Cr3;

                unsafe {
                    if fb_tag.addr == 0xb8000 {
                        ptr += align_up(tag.size as usize, 8);
                        continue;
                    }

                    rprint!("Mapping framebuffer\n");
                    let virt_base = 0xffff_8000_0000_0000u64 + fb_tag.addr as u64;

                    //crate::mem::pmm::map_framebuffer(fb_tag.addr as u64, 0xffff_8000_0000_0000 + fb_tag.addr as u64);
                    //crate::mem::pmm::map_framebuffer(fb_tag.addr as u64, virt_base);
                    crate::mem::pmm::map_framebuffer(0xfd00_0000, 0xffff_8000_fd00_0000);

                    let fb_ptr = 0xffff_8000_fd00_0000 as *mut u64;

                    *fb_ptr = 0xFFFFFFFF; 

                    draw_rect(fb_ptr, 150, 150, 100, 100, 4096, 0x00ffffff);
                    draw_rect(fb_ptr, 250, 250, 100, 100, 4096, 0x00ff0000);
                    draw_rect(fb_ptr, 350, 350, 100, 100, 4096, 0x0000ff00);
                    draw_rect(fb_ptr, 450, 450, 100, 100, 4096, 0x000000ff);

                    if let Some(font) = parse_psf(super::font::PSF_FONT) {
                        draw_text_psf("[guest@rou2ex:/] > ", &font, 25, 30, 0x0000ff00, fb_ptr, fb_tag.pitch as usize, fb_tag.bpp as usize);
                        draw_text_psf("[guest@rou2ex:/] > ", &font, 25, 50, 0x00ffd700, fb_ptr, fb_tag.pitch as usize, fb_tag.bpp as usize);

                        //draw_char("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 35, 35, fb_ptr, fb_tag.pitch as usize, 0xdeadbeef, FONT_RAW);
                    }

                    //draw_test_char(35, 35, fb_ptr);
                    //draw_text_psf("ABCDEFGHIJKLMNOPQRSTUVWXYZ",&FONT_RAW, 35, 35, 0x00ff00, fb_ptr, fb_tag.pitch, fb_tag.bpp);
                }

                //dump_debug_log_to_file();
	*/
}



fn align_up(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}


pub unsafe fn draw_rect(ptr: *mut u64, x0: usize, y0: usize, w: usize, h: usize, pitch: usize, color: u32) {
    for y in y0..(y0 + h) {
        for x in x0..(x0 + w) {
            let offset = y * (pitch / 4) + x;

            ptr.add(offset).write_volatile(color as u64);
        }
    }
}


