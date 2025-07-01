use crate::{fs::fat12::{block::Floppy, fs::Filesystem}, init::config::PATH_CLUSTER};
use super::engine::Point;

pub const MAX_WALLS: usize = 128;

pub fn load_level_from_file<const N: usize>(filename: &[u8; 7]) -> ([Point; N], usize) {
    let mut buf = [0u8; 512];
    let mut walls = [Point { x: 0, y: 0 }; N];
    let mut count = 0;

    let floppy = Floppy::init();

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            unsafe {
                fs.for_each_entry(PATH_CLUSTER, |entry| {
                    if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x10 != 0 {
                        return;
                    }

                    if entry.name.starts_with(filename) && entry.ext.starts_with(b"TXT") {
                        fs.read_file(entry.start_cluster, &mut buf);
                    }
                });
            }
        }
        Err(e) => {}
    }

    let mut i = 0;
    let size = 512;

    while i < size && count < N {
        let mut x = 0usize;
        let mut y = 0usize;

        // Parse x
        while i < size && buf[i] != b',' {
            if buf[i].is_ascii_digit() {
                x = x * 10 + (buf[i] - b'0') as usize;
            }
            i += 1;
        }
        i += 1; // skip ','

        // Parse y
        while i < size && buf[i] != b'\n' {
            if buf[i].is_ascii_digit() {
                y = y * 10 + (buf[i] - b'0') as usize;
            }
            i += 1;
        }
        i += 1; // skip '\n'

        if x < 80 && y < 25 {
            walls[count] = Point { x, y };
            count += 1;
        }
    }

    (walls, count)
}

pub fn load_level_by_number<const N: usize>(level_number: u8) -> ([Point; N], usize) {
    let mut filename = *b"LEVEL00";

    // Convert level number into two-digit ASCII (01 to 99)
    let tens = b'0' + (level_number / 10);
    let ones = b'0' + (level_number % 10);
    filename[5] = tens;
    filename[6] = ones;

    load_level_from_file::<N>(&filename)
}

