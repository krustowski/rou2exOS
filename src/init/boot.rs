use x86_64::VirtAddr;

use crate::{debug::dump_debug_log_to_file, init::{config::{p1_fb_table, p1_fb_table_2, p2_fb_table, p3_fb_table, p4_table}, }, vga::{
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
    typ: u32,           // = 6
    size: u32,          // size of this tag including entries
    entry_size: u32,    // size of each entry (usually 24 bytes)
    entry_version: u32, // usually 0
                        
}

#[repr(C, packed)]
#[derive(Debug)]
struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    typ: u32,       // 1 = usable RAM
    reserved: u32,  // must be 0
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

pub unsafe fn parse_multiboot2_info(vga_index: &mut isize, base_addr: usize) -> usize {
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

                let start = *((ptr + 8) as *const u32);
                let end = *((ptr + 12) as *const u32);
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

                use core::ptr;

                unsafe {
                    let fb_ptr = 0xFFFF_FF80_0000_0000 as *mut u32;

                    /*super::video::map_framebuffer(
                        fb_tag.addr,                       
                        0xFFFF_FF80_0000_0000,               
                        8 * 1024 * 1024,                  
                        &mut P4_TABLE,
                        &mut P3_FB,
                        &mut [&mut P2_FB_0, &mut P2_FB_1],
                        &mut [
                        &mut P1_FB_0, &mut P1_FB_1,
                        &mut P1_FB_2, &mut P1_FB_3,
                        &mut P1_FB_4, &mut P1_FB_5,
                        &mut P1_FB_6, &mut P1_FB_7,
                        &mut P1_FB_8, &mut P1_FB_9,
                        &mut P1_FB_10, &mut P1_FB_11,
                        &mut P1_FB_12, &mut P1_FB_13,
                        &mut P1_FB_14, &mut P1_FB_15,
                        ],
                    );*/

                    x86_64::instructions::tlb::flush_all();

                    let fb_size = (fb_tag.pitch * fb_tag.height) as usize;
                    //let fb = unsafe { core::slice::from_raw_parts_mut(fb_ptr, fb_size) };

                    for y in 0..fb_tag.height  {
                      for x in 0..fb_tag.width  {
                      let offset = y * fb_tag.pitch + x * (fb_tag.bpp as u32 / 8);
                      let pixel = fb_ptr.add(offset as usize);

                      ptr::write(pixel, 0x00);                    
                      ptr::write(pixel.add(1), 0xFF);      
                      ptr::write(pixel.add(2), 0x00);      
                      ptr::write(pixel.add(3), 0xFF);      
                      }
                   }
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

static mut P4_TABLE: [u64; 512] = [0; 512];
static mut P3_FB: [u64; 512] = [0; 512];
static mut P2_FB_0: [u64; 512] = [0; 512];
static mut P2_FB_1: [u64; 512] = [0; 512];
static mut P1_FB_0: [u64; 512] = [0; 512];
static mut P1_FB_1: [u64; 512] = [0; 512];
static mut P1_FB_2: [u64; 512] = [0; 512];
static mut P1_FB_3: [u64; 512] = [0; 512];
static mut P1_FB_4: [u64; 512] = [0; 512];
static mut P1_FB_5: [u64; 512] = [0; 512];
static mut P1_FB_6: [u64; 512] = [0; 512];
static mut P1_FB_7: [u64; 512] = [0; 512];
static mut P1_FB_8: [u64; 512] = [0; 512];
static mut P1_FB_9: [u64; 512] = [0; 512];
static mut P1_FB_10: [u64; 512] = [0; 512];
static mut P1_FB_11: [u64; 512] = [0; 512];
static mut P1_FB_12: [u64; 512] = [0; 512];
static mut P1_FB_13: [u64; 512] = [0; 512];
static mut P1_FB_14: [u64; 512] = [0; 512];
static mut P1_FB_15: [u64; 512] = [0; 512];

