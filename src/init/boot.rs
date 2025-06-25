use crate::{debug::dump_debug_log_to_file, init::{config::{p1_fb_table, p1_fb_table_2, p2_fb_table, p3_fb_table, p4_table}, font::{draw_char, draw_test_char, draw_text, draw_text_psf, parse_psf, FONT_RAW}}, mem, vga::{
    buffer::Color, write::{newline, number, string}
} };
use super::{result::InitResult};

pub fn print_info(vga_index: &mut isize, multiboot_ptr: u64) -> InitResult {
    unsafe {
        debug!("Multiboot2 pointer: ");
        debugn!(multiboot_ptr);
        debugln!("");

        if parse_multiboot2_info(vga_index, (multiboot_ptr as u32) as usize) > 0 {
            return InitResult::Passed;
        }
    }

    debug!("Multiboot2 pointer: ");
    debugn!(multiboot_ptr);
    debugln!("");

    InitResult::Failed
}

#[repr(C)]
#[derive(Debug)]
pub struct TagHeader {
    pub typ: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug)]
struct MemoryMapTag {
    typ: u32,       
    size: u32,          
    entry_size: u32,
    entry_version: u32, 
                        
}

#[repr(C, packed)]
#[derive(Debug)]
struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    typ: u32,   
    reserved: u32,  
}

#[derive(Clone,Copy)]
#[repr(C, packed)]
pub struct FramebufferTag {
    typ: u32,
    pub size: u32,
    pub addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    fb_type: u8,
    reserved: u16,
}

pub unsafe fn parse_multiboot2_info(_vga_index: &mut isize, base_addr: usize) -> usize {
    // Ensure alignment (Multiboot2 requires 8-byte aligned structure)
    let addr = align_up(base_addr, 8);

    // First 4 bytes: total size of the multiboot info
    let total_size = *(addr as *const u32) as usize;

    let mut ptr = addr + 8;
    let end = addr + total_size;

    let mut tag_count = 0;

    while ptr < end {
        let tag = &*(ptr as *const TagHeader);
        if tag.size < 8 || tag.size > 4096 {
            debugln!("Invalid tag size: abort");
            break;
        }

        match tag.typ {
            0 => {
                debugln!("End tag found");
                break;
            }
            1 => {
                debug!("Boot command line tag: ");

                let str_ptr = ptr + 8;
                let str_len = tag.size as usize - 8;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

                let cmdline = core::str::from_utf8_unchecked(raw_bytes);
                debugln!(cmdline);
            }
            3 => {
                debug!("Module tag found: ");

                //let start = *((ptr + 8) as *const u32);
                //let end = *((ptr + 12) as *const u32);
                let str_ptr = ptr + 16;
                let str_len = tag.size as usize - 16;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

                let cmdline = core::str::from_utf8_unchecked(raw_bytes);
                debugln!(cmdline);
            }
            6 => {
                debugln!("Memory map tag");

                let mmap_tag = &*(ptr as *const MemoryMapTag);
                let entries_start = (addr + core::mem::size_of::<MemoryMapTag>()) as *const u8;
                let entry_size = mmap_tag.entry_size as usize;

                if entry_size > 0 {
                    let entries_count = (mmap_tag.size as usize - core::mem::size_of::<MemoryMapTag>()) / entry_size;

                    for i in 0..entries_count {
                        let entry_ptr = entries_start.add(i * entry_size) as *const MemoryMapEntry;
                        let entry = &*entry_ptr;

                        if entry.typ == 1 {
                            debug!("Usable memory region: ");
                            debugn!(entry.base_addr as u64);
                            debug!(" - ");
                            debugn!(entry.length as u64);
                            debugln!(" B");
                        }
                    }
                }
            }
            8 => {
                debugln!("Framebuffer tag: ");

                let fb_tag = &*(ptr as *const FramebufferTag);

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
                    let p4_ptr = &p4_table as *const _ as *mut u64;

                    let p4_virt = &p4_table as *const _ as usize;
                    let p4_phys = p4_virt;

                    //

                    let virt_base = 0xFFFF_FF80_0000_0000u64;
                    let fb_ptr = virt_base as *mut u32;

                    let test_ptr = virt_base as *mut u32;
                    *test_ptr = 0xFFFFFFFF; 

                    //crate::mem::pages::identity_map(p4_table as *mut u64, 4 * 1024 * 1024);
                    //crate::mem::pages::identity_map(p4_table as *mut u64, 0x1000);

                    /*crate::mem::pages::map_32mb(
                        p4_ptr, 
                        fb_tag.addr as usize, 
                        virt_base as usize,
                    );*/

                    //x86_64::instructions::tlb::flush_all();
                    //Cr3::write(PhysFrame::from_start_address(PhysAddr::new(p4_phys as u64)).unwrap(), Cr3Flags::empty());


                    /*for y in 0..500  {
                        for x in 0..500  {
                                //let offset = y * fb_tag.pitch + x * (fb_tag.bpp as u32 / 8);
                            let offset = y * fb_tag.pitch / 4 + x;
                            //let color = 0x00ff00ff;

                            fb_ptr.add(offset as usize).write_volatile(0xdeadbeef);
                            fb_ptr.add(offset as usize + 1).write_volatile(0xfefab0);
                            fb_ptr.add(offset as usize + 2).write_volatile(0xdeadbeef);
                        }
                    }*/

                    /*for y in 0..150 {
                        for x in 0..200 {
                            super::font::put_pixel(x, y, 0xdeadbeef, fb_ptr, 4096, 32);
                            }
                            }*/

                    if let Some(font) = parse_psf(FONT_RAW) {
                        draw_text_psf("[guest@rou2ex:/] > ", &font, 25, 30, 0xdeadbeef, fb_ptr, fb_tag.pitch as usize, fb_tag.bpp as usize);

                        //draw_char("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 35, 35, fb_ptr, fb_tag.pitch as usize, 0xdeadbeef, FONT_RAW);
                    }

                    //draw_test_char(35, 35, fb_ptr);
                    //draw_text_psf("ABCDEFGHIJKLMNOPQRSTUVWXYZ",&FONT_RAW, 35, 35, 0x00ff00, fb_ptr, fb_tag.pitch, fb_tag.bpp);
                }

                dump_debug_log_to_file(&mut 0);

            }
            _ => {
                debug!("Unknown tag: ");
                debugn!(tag.typ);
                debugln!("");
            }
        }

        ptr += align_up(tag.size as usize, 8);
        tag_count += 1;
        if tag_count > 64 {
            debugln!("Too many tags, aborting");
            break;
        }
    }

    tag_count
}

fn align_up(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}

