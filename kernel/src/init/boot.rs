use crate::vga;
use crate::init::result::InitResult;

pub fn print_info(vga_index: &mut isize, multiboot_ptr: u64) -> InitResult {
    unsafe {
        /*parse_multiboot2_info(multiboot_ptr as usize, |msg| {
          for b in msg.as_bytes() {
          vga::write::byte(vga_index, *b, vga::buffer::Color::Yellow);
          }
          });*/
        if parse_multiboot2_info(vga_index, (multiboot_ptr as u32) as usize) > 0 {
            return InitResult::Passed;
        }
    }

    vga::write::string(vga_index, b"Multiboot2 pointer: ", vga::buffer::Color::White);
    vga::write::number(vga_index, (multiboot_ptr as u32) as u64);
    vga::write::newline(vga_index);

    InitResult::Failed
}

#[repr(C)]
#[derive(Debug)]
struct TagHeader {
    typ: u32,
    size: u32,
}

#[repr(C)]
#[derive(Debug)]
struct MemoryMapTag {
    typ: u32,           // = 6
    size: u32,          // size of this tag including entries
    entry_size: u32,    // size of each entry (usually 24 bytes)
    entry_version: u32, // usually 0
                        // followed by [MemoryMapEntry]...
}

#[repr(C, packed)]
#[derive(Debug)]
struct MemoryMapEntry {
    base_addr: u64,
    length: u64,
    typ: u32,       // 1 = usable RAM
    reserved: u32,  // must be 0
}

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
    // followed by palette or color info
}

//pub unsafe fn parse_multiboot2_info(base_addr: usize, mut log_fn: impl FnMut(&str)) {
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
            //log_fn("  Invalid tag size, breaking");
            break;
        }

        match tag.typ {
            0 => {
                //log_fn("  End tag found.");
                break;
            }
            1 => {
                //log_fn("  Boot command line tag");
                let str_ptr = ptr + 8;
                let str_len = tag.size as usize - 8;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

                let cmdline = core::str::from_utf8_unchecked(raw_bytes);
                //log_fn(cmdline);
            }
            3 => {
                //log_fn("  Module tag");
                let start = *((ptr + 8) as *const u32);
                let end = *((ptr + 12) as *const u32);
                let str_ptr = ptr + 16;
                let str_len = tag.size as usize - 16;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);

                let cmdline = core::str::from_utf8_unchecked(raw_bytes);
                //log_fn(cmdline);
            }
            6 => {
                //log_fn("  Memory map");
                let mmap_tag = &*(ptr as *const MemoryMapTag);
                let entries_start = (addr + core::mem::size_of::<MemoryMapTag>()) as *const u8;
                let entry_size = mmap_tag.entry_size as usize;

                if entry_size > 0 {
                    let entries_count = (mmap_tag.size as usize - core::mem::size_of::<MemoryMapTag>()) / entry_size;

                    for i in 0..entries_count {
                        let entry_ptr = entries_start.add(i * entry_size) as *const MemoryMapEntry;
                        let entry = &*entry_ptr;

                        if entry.typ == 1 {
                            /*vga::write::string(vga_index, b"Usable mem region: ", vga::buffer::Color::White);
                            vga::write::number(vga_index, entry.base_addr as u64);
                            vga::write::string(vga_index, b" - ", vga::buffer::Color::White);
                            vga::write::number(vga_index, entry.length as u64);
                            vga::write::newline(vga_index);*/
                        }
                    }
                }
            }
            8 => {
                let fb_tag = &*(ptr as *const FramebufferTag);

                /*vga::write::string(vga_index, b"Frame buf (bpp + res): ", vga::buffer::Color::White);
                vga::write::number(vga_index, fb_tag.bpp as u64);
                vga::write::string(vga_index, b" + ", vga::buffer::Color::White);
                vga::write::number(vga_index, fb_tag.width as u64);
                vga::write::string(vga_index, b"x", vga::buffer::Color::White);
                vga::write::number(vga_index, fb_tag.height as u64);
                vga::write::newline(vga_index);*/

            }
            _ => {
                //log_fn("  Unknown tag")
                //log_fn(&format!("  Unknown tag: type={}, size={}", tag.typ, tag.size));
            }
        }

        ptr += align_up(tag.size as usize, 8);
        tag_count += 1;
        if tag_count > 64 {
            //log_fn("  Too many tags, stopping");
            break;
        }
    }

    tag_count
}

fn align_up(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}

