use crate::fs::fat12::{block::Floppy, fs::Filesystem};
use crate::video::sysprint::{Result};
use crate::init::config::{PATH_CLUSTER, set_path};

pub fn floppy_check_init() -> Result {
    let floppy = Floppy::init();

    let res = match Filesystem::new(&floppy) {
        Ok(_) => {
            Result::Passed
        }
        Err(e) => {
            debug!("Filesystem init (floppy) fail: ");
            debugln!(e);
            Result::Skipped
        }
    };

    set_path(b"/");

    unsafe {
        PATH_CLUSTER = 0;
    }

    res
}  

/*pub fn print_info(vga_index: &mut isize) {
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
}*/
