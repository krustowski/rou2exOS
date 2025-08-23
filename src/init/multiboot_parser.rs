pub const MULTIBOOT_HEADER: u32 = 1;

pub const MULTIBOOT_SEARCH: u32 = 32768;

pub const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe85250d6;
pub const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36d76289;

pub const MULTIBOOT_MOD_ALIGN: u32 = 0x00001000;
pub const MULTIBOOT_INFO_ALIGN: u32 = 0x00000008;
pub const MULTIBOOT_TAG_ALIGN: u32 = 8;
pub const MULTIBOOT_HEADER_ALIGN: u8 = 8;

pub enum MULTIBOOT_TAG_TYPE {
    MULTIBOOT_TAG_TYPE_END              = 0,
    MULTIBOOT_TAG_TYPE_CMDLINE          = 1,
    MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME = 2,
    MULTIBOOT_TAG_TYPE_MODULE           = 3,
    MULTIBOOT_TAG_TYPE_BASIC_MEMINFO    = 4,
    MULTIBOOT_TAG_TYPE_BOOTDEV          = 5,
    MULTIBOOT_TAG_TYPE_MMAP             = 6,
    MULTIBOOT_TAG_TYPE_VBE              = 7,
    MULTIBOOT_TAG_TYPE_FRAMEBUFFER      = 8,
    MULTIBOOT_TAG_TYPE_ELF_SECTIONS     = 9,
    MULTIBOOT_TAG_TYPE_APM              = 10,
    MULTIBOOT_TAG_TYPE_EFI32            = 11,
    MULTIBOOT_TAG_TYPE_EFI64            = 12,
    MULTIBOOT_TAG_TYPE_SMBIOS           = 13,
    MULTIBOOT_TAG_TYPE_ACPI_OLD         = 14,
    MULTIBOOT_TAG_TYPE_ACPI_NEW         = 15,
    MULTIBOOT_TAG_TYPE_NETWORK          = 16,
    MULTIBOOT_TAG_TYPE_EFI_MMAP         = 17,
    MULTIBOOT_TAG_TYPE_EFI_BS           = 18,
    MULTIBOOT_TAG_TYPE_EFI32_IH         = 19,
    MULTIBOOT_TAG_TYPE_EFI64_IH         = 20,
    MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR   = 21,
}

pub enum MULTIBOOT_HEADER_TAG {
    MULTIBOOT_HEADER_TAG_END  = 0,
    MULTIBOOT_HEADER_TAG_INFORMATION_REQUEST = 1,
    MULTIBOOT_HEADER_TAG_ADDRESS = 2,
    MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS = 3,
    MULTIBOOT_HEADER_TAG_CONSOLE_FLAGS = 4,
    MULTIBOOT_HEADER_TAG_FRAMEBUFFER = 5,
    MULTIBOOT_HEADER_TAG_MODULE_ALIGN = 6,
    MULTIBOOT_HEADER_TAG_EFI_BS       = 7,
    MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI32 = 8,
    MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI64 = 9,
    MULTIBOOT_HEADER_TAG_RELOCATABLE = 10,
}

pub enum MULTIBOOT_MEMORY {
    MULTIBOOT_MEMORY_AVAILABLE              = 1,
    MULTIBOOT_MEMORY_RESERVED               = 2,
    MULTIBOOT_MEMORY_ACPI_RECLAIMABLE       = 3,
    MULTIBOOT_MEMORY_NVS                    = 4,
    MULTIBOOT_MEMORY_BADRAM                 = 5,
}

pub const MULTIBOOT_ARCHITECTURE_I386: u8 = 0;
pub const MULTIBOOT_ARCHITECTURE_MIPS32: u8 =  4;
pub const MULTIBOOT_HEADER_TAG_OPTIONAL: u8 = 1;

pub enum MULTIBOOT_LOAD_PREFERENCE {
    MULTIBOOT_LOAD_PREFERENCE_NONE = 0,
    MULTIBOOT_LOAD_PREFERENCE_LOW  = 1,
    MULTIBOOT_LOAD_PREFERENCE_HIGH = 2,
}

const MULTIBOOT_CONSOLE_FLAGS_CONSOLE_REQUIRED: u8 = 1;
const MULTIBOOT_CONSOLE_FLAGS_EGA_TEXT_SUPPORTED: u8 = 2;


use crate::{debug::dump_debug_log_to_file, init::{config::{p1_fb_table, p1_fb_table_2, p2_fb_table, p3_fb_table, p4_table}, font::{draw_text_psf, parse_psf}}, mem, vga::{
    buffer::Color, write::{newline, number, string}
} };
use super::{result::InitResult};

pub fn print_info(multiboot_ptr: u64, mut fb_tag: &FramebufferTag) -> InitResult {
    unsafe {
        debug!("Multiboot2 pointer: ");
        debugn!(multiboot_ptr);
        debugln!("");

        if parse_multiboot2_info((multiboot_ptr as u32) as usize, fb_tag) > 0 {
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
    pub typ: MULTIBOOT_TAG_TYPE,
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

#[derive(Clone,Copy,Default)]
#[repr(C, packed)]
pub struct FramebufferTag {
    pub typ: u32,
    pub size: u32,
    pub addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub fb_type: u8,
    pub reserved: u16,
}

#[repr(C, packed)]
pub struct AcpiRSDPTag {
    pub typ: u32,
    pub size: u32,
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_addr: u32,
}

#[repr(C, packed)]
pub struct AcpiSDTHeader {
    pub signature: [u8; 4], //array
    pub length: u32,
    pub revision: u8, 
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creatpr_revision: u32,
}

#[repr(C, packed)] //directive?? status kinda idfk
pub struct UsableMemory {
	start: u64,
	end: u64,
	count: u8,

}




static mut U_MEM: UsableMemory = UsableMemory{start: 0, end: 0, count: 0}; //change this accordingly!!! placeholder for now

//&&&&&&& reference variable borrower cannot change 
//usize like size_t from C
//main parser
pub unsafe fn parse_multiboot2_info(base_addr: usize, mut fb_tag: &FramebufferTag) -> usize {
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
            MULTIBOOT_TAG_TYPE::MULTIBOOT_TAG_TYPE_END => {
                debugln!("End tag found");
                break;
            }

            MULTIBOOT_TAG_TYPE::MULTIBOOT_TAG_TYPE_CMDLINE => {

                debug!("Boot command line tag: ");

                let str_ptr = ptr + 8;
                let str_len = tag.size as usize - 8;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

                let cmdline = core::str::from_utf8_unchecked(raw_bytes);
                debugln!(cmdline);
            }

            MULTIBOOT_TAG_TYPE::MULTIBOOT_TAG_TYPE_MODULE => { 
                debug!("Module tag found: ");

                //let start = *((ptr + 8) as *const u32);
                //let end = *((ptr + 12) as *const u32);
                let str_ptr = ptr + 16;
                let str_len = tag.size as usize - 16;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

                let cmdline = core::str::from_utf8_unchecked(raw_bytes);
                debugln!(cmdline);
            }

            MULTIBOOT_TAG_TYPE::MULTIBOOT_TAG_TYPE_MMAP => {
				//ptr as *const foo ; immutable raw pointer
                let mmap_tag = &*(ptr as *const MemoryMapTag);
				memory_map_tag(mmap_tag); 
            }

            MULTIBOOT_TAG_TYPE::MULTIBOOT_TAG_TYPE_FRAMEBUFFER => {
                debugln!("Framebuffer tag: ");

                fb_tag = &*(ptr as *const FramebufferTag);

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

            }

            MULTIBOOT_TAG_TYPE::MULTIBOOT_TAG_TYPE_ACPI_OLD => {
                debugln!("ACPI v1 Root System Descriptor Pointer tag");

                let acpi_tag = &*(ptr as *const AcpiRSDPTag);
                debug!("Signature: ");
                debug!(acpi_tag.signature);
                debug!("\nOEM: ");
                debug!(acpi_tag.oemid);
                debugln!("");

                let acpi_sdt = &*(acpi_tag.rsdt_addr as *const AcpiSDTHeader);
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

pub unsafe fn memory_map_tag(mmap_tag: &MemoryMapTag) {
	debugln!("Memory map tag");
	let entries_start = (mmap_tag + core::mem::size_of::<MemoryMapTag>()) as *const u8; //wont compile look into this
    let entry_size = mmap_tag.entry_size as usize;
	let end 



}




pub unsafe fn boot_line_tag() {

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


