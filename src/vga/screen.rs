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

pub fn scroll_at(vga_index: &mut isize, height: &mut isize) {
    if *height == 0 {
        *height = buffer::HEIGHT as isize;
    }

    if (*vga_index / 2) / (buffer::WIDTH as isize) < *height {
        return;
    }

    unsafe {
        let row_size = buffer::WIDTH * 2; // bytes per row
        let screen_size = row_size * buffer::HEIGHT;

        // Copy all rows up one line: from row 1 to row 0
        ptr::copy(
            buffer::VGA_BUFFER.add(row_size), // start of row 1
            buffer::VGA_BUFFER,               // start of row 0
            row_size * (buffer::HEIGHT - 1),  // total bytes of 24 rows
        );

        let last_line_offset = row_size * (buffer::HEIGHT - 1);
        let last_line_ptr = buffer::VGA_BUFFER.add(last_line_offset);

        for i in 0..buffer::WIDTH {
            *last_line_ptr.add(i * 2) = b' ';
            *last_line_ptr.add(i * 2 + 1) = 0x07; 
        }
    }

    *vga_index = (*height as isize - 1) * buffer::WIDTH as isize * 2;
}

pub fn scroll(vga_index: &mut isize) {
    if (*vga_index / 2) / (buffer::WIDTH as isize) < (buffer::HEIGHT as isize) {
        return;
    }

    unsafe {
        let row_size = buffer::WIDTH * 2; // bytes per row
        let screen_size = row_size * buffer::HEIGHT;

        // Copy all rows up one line: from row 1 to row 0
        ptr::copy(
            buffer::VGA_BUFFER.add(row_size), // start of row 1
            buffer::VGA_BUFFER,               // start of row 0
            row_size * (buffer::HEIGHT - 1),  // total bytes of 24 rows
        );

        let last_line_offset = row_size * (buffer::HEIGHT - 1);
        let last_line_ptr = buffer::VGA_BUFFER.add(last_line_offset);

        for i in 0..buffer::WIDTH {
            *last_line_ptr.add(i * 2) = b' ';
            *last_line_ptr.add(i * 2 + 1) = 0x07; // Light gray on black
        }
    }

    *vga_index = (buffer::HEIGHT as isize - 1) * buffer::WIDTH as isize * 2;
}


