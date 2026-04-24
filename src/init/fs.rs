use crate::fs::fat12::{block::Floppy, fs::Filesystem};
use crate::init::config::SYSTEM_CONFIG;
use crate::video::sysprint::Result;

pub fn floppy_check_init() -> Result {
    let floppy = Floppy::init();

    let res = match Filesystem::new(&floppy) {
        Ok(_) => Result::Passed,
        Err(e) => {
            debug!("Filesystem init (floppy) fail: ");
            debugln!(e);
            Result::Skipped
        }
    };

    if let Some(mut c) = SYSTEM_CONFIG.try_lock() {
        c.set_path(b"/", 0);
    }

    res
}
