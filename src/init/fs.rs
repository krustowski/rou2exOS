use crate::fs::fat12::{block::Floppy, fs::Filesystem};
/*use super::{
    config::{PATH_CLUSTER, set_path},
    result,
};

pub fn check_floppy() -> result::InitResult {
    let floppy = Floppy::init();

    let res = match Filesystem::new(&floppy) {
        Ok(_) => {
            result::InitResult::Passed
        }
        Err(e) => {
            debug!("Filesystem init (floppy) fail: ");
            debugln!(e);
            result::InitResult::Skipped
        }
    };

    set_path(b"/");

    unsafe {
        PATH_CLUSTER = 0;
    }

    res
}  */

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
