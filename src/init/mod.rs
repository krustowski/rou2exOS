pub mod ascii;
pub mod boot;
pub mod color;
pub mod config;
pub mod cpu;
pub mod fs;
pub mod heap;
pub mod result;

pub fn init(vga_index: &mut isize, multiboot_ptr: u64) {
    debugln!("Kernel init start");

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

    result::print_result(
        "Initialize heap allocator", 
        heap::print_result(vga_index),
        vga_index
    );

    result::print_result(
        "Read Multiboot2 tags", 
        boot::print_info(vga_index, multiboot_ptr),
        vga_index
    );

    result::print_result(
        "Check floppy drive", 
        fs::check_floppy(vga_index),
        vga_index
    );

    color::color_demo(vga_index);
    ascii::ascii_art(vga_index);

    crate::audio::midi::play_melody();
}
