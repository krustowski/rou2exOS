use crate::vga;

pub fn ascii_art(vga_index: &mut isize) {
    vga::write::string(vga_index, b"                 ____            ___  _____ ", 0xa);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b" _ __ ___  _   _|___ \\ _____  __/ _ \\/ ____| ", 0xa);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"| '__/ _ \\| | | | __) / _ \\ \\/ / | | \\___ \\", 0xa);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"| | | (_) | |_| |/ __/  __/>  <| |_| |___) |", 0xa);
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"|_|  \\___/ \\__,_|_____\\___/_/\\_\\____/|____/", 0xa);
    vga::write::newline(vga_index);
    vga::write::newline(vga_index);
}
