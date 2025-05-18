pub mod ascii;
pub mod boot;
pub mod color;
pub mod cpu;
pub mod fs;

pub fn init(vga_index: &mut isize, multiboot_ptr: u64) {
    ascii::ascii_art(vga_index);
    cpu::print_mode(vga_index);
    boot::print_info(vga_index, multiboot_ptr);
    color::color_demo(vga_index);
    fs::print_info(vga_index);
}
