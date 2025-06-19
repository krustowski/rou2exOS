use crate::vga::{
    write::{string, newline},
    buffer::Color,
};

pub fn ascii_art(vga_index: &mut isize) {
    string(vga_index, b"                 ____            ___  _____ ", Color::Green);
    newline(vga_index);
    string(vga_index, b" _ __ ___  _   _|___ \\ _____  __/ _ \\/ ____| ", Color::Green);
    newline(vga_index);
    string(vga_index, b"| '__/ _ \\| | | | __) / _ \\ \\/ / | | \\___ \\", Color::Green);
    newline(vga_index);
    string(vga_index, b"| | | (_) | |_| |/ __/  __/>  <| |_| |___) |", Color::Green);
    newline(vga_index);
    string(vga_index, b"|_|  \\___/ \\__,_|_____\\___/_/\\_\\____/|____/", Color::Green);
    newline(vga_index);
    newline(vga_index);
}
