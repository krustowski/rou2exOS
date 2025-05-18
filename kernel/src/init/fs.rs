use crate::fs::block::Floppy;
use crate::fs::fs::Fs;

pub fn print_info(vga_index: &mut isize) {
    let floppy = Floppy;
    Floppy::init();

    crate::vga::write::string(vga_index, b"Reading floppy...", crate::vga::buffer::Color::White);
    crate::vga::write::newline(vga_index);

    let fs = Fs::new(&floppy, vga_index);
    fs.list_root_dir(vga_index);
}
