use core::ptr;
use crate::vga::buffer;

pub fn clear(vga_index: &mut isize) {
    unsafe {
        for row in 0..buffer::HEIGHT {
            for col in 0..buffer::WIDTH {
                let idx = (row * buffer::WIDTH + col) * 2;
                let mut offset: isize = idx as isize;

                *buffer::VGA_BUFFER.offset(offset) = b' '; // Character byte
                offset += 1;
                *buffer::VGA_BUFFER.offset(offset) = 0x07; // Attribute byte
            }
        }

        *vga_index = 0;
    }
}

pub fn scroll(vga_index: &mut isize) {
    if (*vga_index / 2) / 80 < buffer::HEIGHT as isize {
        return;
    }

    unsafe {
        let mut offset: isize = buffer::WIDTH as isize * 2;

        // Copy 24 rows * 80 cols * 2 bytes = 3840 bytes
        ptr::copy(
            buffer::VGA_BUFFER.offset(offset), // from row 1
            buffer::VGA_BUFFER,
            buffer::WIDTH * (buffer::HEIGHT - 1) * 2, // size: 24 rows
        );

        offset = buffer::WIDTH as isize * (buffer::WIDTH as isize - 1) * 2;

        // Clear last line (row 24)
        let last_line = buffer::VGA_BUFFER.offset(offset);
        for i in 0..buffer::WIDTH {
            let mut offset: isize = i as isize * 2;
            *last_line.offset(offset) = b' ';
            offset += 1;
            *last_line.offset(offset) = 0x07; // Light gray on black
        }
    }

    *vga_index = buffer::WIDTH as isize * (buffer::HEIGHT as isize - 1) * 2;
}

