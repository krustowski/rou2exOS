use crate::fs::fat12::Fs;
use crate::vga::{buffer::Color, screen::clear, write::{string, newline}};

static mut TEXT_BUFFER: [[u8; 80]; 25] = [[b' '; 80]; 25];

pub fn run(file: &[u8]) {
    let vga_index: &mut isize = &mut 0;
    clear(vga_index);

    loop {}
}
