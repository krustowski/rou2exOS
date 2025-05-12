pub mod ascii;
pub mod boot;
pub mod color;
pub mod cpu;

pub fn init(vga_index: &mut isize, multiboot_ptr: u32) {
    ascii::ascii_art(vga_index);
    cpu::print_mode(vga_index);
    boot::print_info(vga_index, multiboot_ptr);
    color::color_demo(vga_index);
}
