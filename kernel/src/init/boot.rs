use crate::vga;

pub fn print_info(vga_index: &mut isize, multiboot_ptr: u32) {
    vga::write::string(vga_index, b"Multiboot2 pointer: ", vga::buffer::Color::White);
    vga::write::number(vga_index, multiboot_ptr as u64);

    /*unsafe {
        parse_multiboot2_info(multiboot_ptr as usize, |msg| {
            vga::write::string(vga_index, msg.as_bytes(), vga::buffer::Color::Yellow);
        });
    }*/

    vga::write::newline(vga_index);
}

#[repr(C)]
#[derive(Debug)]
struct TagHeader {
    typ: u32,
    size: u32,
}

pub unsafe fn parse_multiboot2_info(base_addr: usize, mut log_fn: impl FnMut(&str)) {
    // Ensure alignment (Multiboot2 requires 8-byte aligned structure)
    let addr = align_up(base_addr, 8);

    // First 4 bytes: total size of the multiboot info
    let total_size = *(addr as *const u32) as usize;

    let mut ptr = addr + 8;
    let end = addr + total_size;

    let mut tag_count = 0;

    while ptr < end {
        let tag = &*(ptr as *const TagHeader);
        if tag.size < 8 {
            log_fn("  Invalid tag size, breaking");
            break;
        }

        match tag.typ {
            0 => {
                log_fn("  End tag found.");
                break;
            }
            1 => {
                log_fn("  Boot command line tag");
                let str_ptr = ptr + 8;
                let str_len = tag.size as usize - 8;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);
                /*if let Ok(cmdline) = core::str::from_utf8(raw_bytes) {
                    //log_fn(&format!("    Command line: {}", cmdline));
                } else {
                    //log_fn("    [invalid UTF-8]");
                }*/
            }
            3 => {
                log_fn("  Module tag");
                let start = *((ptr + 8) as *const u32);
                let end = *((ptr + 12) as *const u32);
                let str_ptr = ptr + 16;
                let str_len = tag.size as usize - 16;
                let raw_bytes = core::slice::from_raw_parts(str_ptr as *const u8, str_len);
                /*if let Ok(name) = core::str::from_utf8(raw_bytes) {
                    //log_fn(&format!("    Module: 0x{:x}–0x{:x} ({})", start, end, name));
                } else {
                    //log_fn("    Module name: [invalid UTF-8]");
                }*/
            }
            _ => {
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
}

fn align_up(x: usize, align: usize) -> usize {
    (x + align - 1) & !(align - 1)
}

