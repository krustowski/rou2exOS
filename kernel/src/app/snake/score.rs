use crate::{fs::fat12::{block::Floppy, fs::Fs}, init::config::PATH_CLUSTER};

const HIGH_SCORE_FILE: &[u8; 11] = b"SKSCORE BIN";
const SCORE_LEN: usize = 4;

type Error = &'static str;

pub fn save_high_scores_fat12(scores: &[u32; SCORE_LEN], vga_index: &mut isize) -> Result<(), Error> {
    let floppy = Floppy;

    // Serialize scores as little endian u32
    let mut buf = [0u8; 4 * SCORE_LEN];
    for (i, &score) in scores.iter().enumerate() {
        buf[i * 4..i * 4 + 4].copy_from_slice(&score.to_le_bytes());
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let cluster: u16 = 0;

            unsafe {
                /*fs.for_each_entry(PATH_CLUSTER, |entry| {
                    if entry.name[0] == 0x00 || entry.name[0] = 0xE5 || entry.attr & 0x10 != 0 {
                        return;
                    }

                    if entry.name.starts_with(b"SKSCORE") && entry.ext.starts_with(b"BIN") {
                        cluster = entry.start_cluster;
                    }
                }, vga_index);*/

                fs.write_file(PATH_CLUSTER, HIGH_SCORE_FILE, &buf, vga_index);
            }
        }
        Err(e) => {}
    }

    Ok(())
}

pub fn load_high_scores_fat12(vga_index: &mut isize) -> Result<[u32; 5], Error> {
    let mut buf = [0u8; 20]; // 5 × 4 bytes
    let mut sector_buf = [0u8; 512];
    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut cluster: u16 = 0;

            unsafe {
                fs.for_each_entry(PATH_CLUSTER, |entry| {
                    if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x10 != 0 {
                        return;
                    }

                    if entry.name.starts_with(b"SKSCORE") && entry.ext.starts_with(b"BIN") {
                        cluster = entry.start_cluster;
                    }
                }, vga_index);

                if cluster == 0 {
                    fs.write_file(PATH_CLUSTER, HIGH_SCORE_FILE, &buf, vga_index);
                }
            }

            fs.read_file(cluster, &mut sector_buf, vga_index);
        }
        Err(e) => {}
    }

    let mut scores = [0u32; 5];
    for i in 0..5 {
        scores[i] = u32::from_le_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]);
    }

    Ok(scores)
}

