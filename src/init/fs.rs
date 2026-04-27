use crate::fs::fat12::{block::Floppy, fs::Filesystem};
use crate::fs::iso9660::Iso9660;
use crate::fs::vfs::{self, FsType};
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

pub fn vfs_init() {
    vfs::mount(b"/", FsType::Root);
    rprint!("vfs: / mounted (rootfs)\n");

    vfs::mount(b"/mnt/fat", FsType::Fat12);
    rprint!("vfs: /mnt/fat mounted (fat12)\n");

    if Iso9660::probe().is_some() {
        vfs::mount(b"/mnt/iso", FsType::Iso9660);
        rprint!("vfs: /mnt/iso mounted (iso9660)\n");
    }
}
