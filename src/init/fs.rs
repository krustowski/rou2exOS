use crate::fs::fat12::{block::Floppy, fs::Fs};
use super::result;
use super::config::{self, PATH_CLUSTER};

pub fn check_floppy(vga_index: &mut isize) -> result::InitResult {
    let floppy = Floppy;
    Floppy::init();

    let res: result::InitResult;

    match Fs::new(&floppy, vga_index) {
        Ok(_) => {
            res = result::InitResult::Passed
        }
        Err(_) => {
            res = result::InitResult::Skipped
        }
    }

    crate::init::config::set_path(b"/");

    unsafe {
        crate::init::config::PATH_CLUSTER = 0;
    }

    res
} 

pub fn print_info(vga_index: &mut isize) {
    let floppy = Floppy;
    Floppy::init();

    crate::vga::write::string(vga_index, b"Reading floppy...", crate::vga::buffer::Color::White);
    crate::vga::write::newline(vga_index);

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            unsafe {
                fs.list_dir(PATH_CLUSTER, &[b' '; 11], vga_index);
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}
