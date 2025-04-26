use core::ptr;
use crate::vga::buffer;

pub fn clear(vga_index: &mut isize) {
    unsafe {
        for row in 0..buffer::HEIGHT {
            for col in 0..buffer::WIDTH {
                let idx = (row * buffer::WIDTH + col) * 2;
                *buffer::VGA_BUFFER.offset(idx as isize) = b' '; // Character byte
                *buffer::VGA_BUFFER.offset(idx as isize + 1) = 0x07; // Attribute byte
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
        // Copy 24 rows * 80 cols * 2 bytes = 3840 bytes
        ptr::copy(
            buffer::VGA_BUFFER.offset((buffer::WIDTH * 2) as isize), // from row 1
            buffer::VGA_BUFFER,
            (buffer::WIDTH * (buffer::HEIGHT - 1) * 2) as usize, // size: 24 rows
        );

        // Clear last line (row 24)
        let last_line = buffer::VGA_BUFFER.offset((buffer::WIDTH * (buffer::HEIGHT - 1) * 2) as isize);
        for i in 0..buffer::WIDTH {
            *last_line.offset((i * 2) as isize) = b' ';
            *last_line.offset((i * 2 + 1) as isize) = 0x07; // Light gray on black
        }
    }

    *vga_index = buffer::WIDTH as isize * (buffer::HEIGHT as isize - 1) * 2;
}

