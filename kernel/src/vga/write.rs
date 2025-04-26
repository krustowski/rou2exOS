use crate::vga::buffer;
use crate::vga::screen;

/// Write a whole string to screen
pub fn string(vga_index: &mut isize, string: &[u8], color: u8) {
    screen::scroll(vga_index);

    for &byte in string {
        unsafe {
            *buffer::VGA_BUFFER.offset(*vga_index) = byte;
            *buffer::VGA_BUFFER.offset(*vga_index + 1) = color;
            *vga_index += 2;
        }
    }
}

/// Move to a new line
pub fn newline(vga_index: &mut isize) {
    // VGA 80x25: each line is 80 chars * 2 bytes per char
    *vga_index += (80 * 2) - (*vga_index % (80 * 2));
}


