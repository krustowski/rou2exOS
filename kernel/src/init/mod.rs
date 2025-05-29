pub mod ascii;
pub mod boot;
pub mod color;
pub mod config;
pub mod cpu;
pub mod fs;
pub mod result;

pub fn init(vga_index: &mut isize, multiboot_ptr: u64) {
    result::print_result(
        "Load kernel", 
        result::InitResult::Passed,
        vga_index
    );

    result::print_result(
        "Check 64-bit Long Mode", 
        cpu::check_mode(),
        vga_index
    );

    /*result::print_result(
        "Check multiboot2 tag count", 
        boot::print_info(vga_index, multiboot_ptr),
        vga_index
    );*/

    result::print_result(
        "Check floppy drive", 
        fs::check_floppy(vga_index),
        vga_index
    );

    color::color_demo(vga_index);
    ascii::ascii_art(vga_index);

    crate::sound::midi::play_melody();
}
