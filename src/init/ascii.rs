use crate::vga;

pub fn ascii_art(vga_index: &mut isize) {
    vga::write::string(vga_index, b"                 ____            ___  _____ ", vga::buffer::Color::Green);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b" _ __ ___  _   _|___ \\ _____  __/ _ \\/ ____| ", vga::buffer::Color::Green);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"| '__/ _ \\| | | | __) / _ \\ \\/ / | | \\___ \\", vga::buffer::Color::Green);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"| | | (_) | |_| |/ __/  __/>  <| |_| |___) |", vga::buffer::Color::Green);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"|_|  \\___/ \\__,_|_____\\___/_/\\_\\____/|____/", vga::buffer::Color::Green);
    vga::write::newline(vga_index);
    vga::write::newline(vga_index);
}
