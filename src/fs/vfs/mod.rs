pub struct FsOps {}

pub struct Mountpoint {
    pub mountpoint: [u8; 8],
    pub fs_type: [u8; 8],
    pub fs_ops: FsOps,
}

impl Mountpoint {
    fn new(mountpoint: [u8; 8], fs_type: [u8; 8], fs_ops: FsOps) -> Self {
        Self {
            mountpoint,
            fs_type,
            fs_ops,
        }
    }
}

const MAX_MOUNTPOINTS: usize = 2;

/*static mut MOUNTPOINTS: [Mountpoint; MAX_MOUNTPOINTS] = [
    Mountpoint::new(b"/       ", b"ROOT    ", FsOps {}),
    Mountpoint::new(b"floppy  ", b"FAT12   ", FsOps {}),
];*/

trait FilesystemOps {
    fn open(&self);
}
