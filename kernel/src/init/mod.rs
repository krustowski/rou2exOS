pub mod ascii;
pub mod color;
pub mod cpu;

pub fn init(vga_index: &mut isize) {
    ascii::ascii_art(vga_index);
    cpu::print_mode(vga_index);
    color::color_demo(vga_index);
}
