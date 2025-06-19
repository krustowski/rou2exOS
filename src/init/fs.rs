use crate::fs::fat12::{block::Floppy, fs::Fs};
use crate::vga::{
    write::{string, newline},
    buffer::Color,
};
use super::{
    config::{PATH_CLUSTER, set_path},
    result,
};

pub fn check_floppy(vga_index: &mut isize) -> result::InitResult {
    let floppy = Floppy;
    Floppy::init();

    let res: result::InitResult;

    match Fs::new(&floppy, vga_index) {
        Ok(_) => {
            res = result::InitResult::Passed
        }
        Err(e) => {
            debug!("Filesystem init (floppy) fail: ");
            debugln!(e);
            res = result::InitResult::Skipped
        }
    }

    set_path(b"/");

    unsafe {
        PATH_CLUSTER = 0;
    }

    res
} 

pub fn print_info(vga_index: &mut isize) {
    let floppy = Floppy;
    Floppy::init();

    string(vga_index, b"Reading floppy...", Color::White);
    newline(vga_index);

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            unsafe {
                fs.list_dir(PATH_CLUSTER, &[b' '; 11], vga_index);
            }
        }
        Err(e) => {
            string(vga_index, e.as_bytes(), Color::Red);
            newline(vga_index);
        }
    }
}
