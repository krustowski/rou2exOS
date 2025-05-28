use crate::{
    app::snake::menu::{draw_menu, draw_window}, fs::fat12::{
        block::Floppy, 
        fs::Fs}, init::config::PATH_CLUSTER, input::keyboard::keyboard_read_scancode, slice_end_index_len_fail, vga::{buffer::VGA_BUFFER, write::{byte, newline, number, string}}
};

const HIGH_SCORE_FILE: &[u8; 11] = b"SKSCORE BIN";
const SCORE_LEN: usize = 5;

type Error = &'static str;

pub fn render_scores_window(scores: &[u32; SCORE_LEN], vga_index: &mut isize) {
    let mut menu: [&str; SCORE_LEN] = [""; SCORE_LEN];

    static mut buf0: [u8; 32] = [0u8; 32];
    static mut buf1: [u8; 32] = [0u8; 32];
    static mut buf2: [u8; 32] = [0u8; 32];
    static mut buf3: [u8; 32] = [0u8; 32];
    static mut buf4: [u8; 32] = [0u8; 32];
    
    unsafe {
        menu[0] = sprintf_score(b"1. ", &mut buf0, scores[0]);
        menu[1] = sprintf_score(b"2. ", &mut buf1, scores[1]);
        menu[2] = sprintf_score(b"3. ", &mut buf2, scores[2]);
        menu[3] = sprintf_score(b"4. ", &mut buf3, scores[3]);
        menu[4] = sprintf_score(b"5. ", &mut buf4, scores[4]);
    }

    draw_window(25, 5, 30, 15, Some("High Scores"));
    draw_menu(31, 8, &menu);
    
    loop {
        let scancode = keyboard_read_scancode();

        if scancode == 0x01 {
            break;
        }
    }
}

pub fn save_high_scores_fat12(scores: &[u32; SCORE_LEN], vga_index: &mut isize) -> Result<(), Error> {
    let floppy = Floppy;

    // Serialize scores as little endian u32
    let mut buf = [0u8; 4 * SCORE_LEN];

    for (i, &score) in scores.iter().enumerate() {

        if i >= buf.len() {
            break;
        }

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

pub fn load_high_scores_fat12(vga_index: &mut isize) -> Option<[u32; SCORE_LEN]> {
    let floppy = Floppy;
    let mut sector_buf = [0u8; 512];

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
            }

            if cluster == 0 {
                return None;
            }

            fs.read_file(cluster, &mut sector_buf, vga_index);
        }
        Err(_) => return None
    }

    let scores = parse_scores_from_sector(&sector_buf);
    scores
}

pub fn parse_scores_from_sector(sector_buf: &[u8; 512]) -> Option<[u32; 5]> {
    let mut scores = [0u32; SCORE_LEN];

    // Manually extract 5 * 4 = 20 bytes without causing bounds checks
    for i in 0..SCORE_LEN - 1 {
        let offset = i * 4;

        // Bounds check manually before copying
        if offset + 4 > sector_buf.len() {
            return None;
        }

        // Copy the 4 bytes manually without panic
        let mut b = [0u8; 4];
        for j in 0..3 {
            if let Some(byte) = sector_buf.get(offset + j) {
                if let Some(sl) = b.get_mut(j) {
                    *sl = *byte;
                }
            }
        }

        if i >= scores.len() {
            break;
        }

        if let Some(sc) = scores.get_mut(i) {
            *sc = u32::from_le_bytes(b);
        }
    }

    Some(scores)
}

pub fn sprintf_score<'a>(prefix: &'static [u8], buf: &'a mut [u8], score: u32) -> &'a str {
    let mut i = 0;

    for &b in prefix {
        if i < buf.len() {
            if let Some(bf) = buf.get_mut(i) {
                *bf = b;
            }
            i += 1;
        }
    }

    let mut num = score;
    if num == 0 {
        if i < buf.len() {
            if let Some(bf) = buf.get_mut(i) {
                *bf = b'0'
            }
            i += 1;
        }
    } else {
        let mut temp = [0u8; 10];
        let mut j = 0;
        while num > 0 && j < temp.len() {
            if let Some(tmp) = temp.get_mut(j) {
                *tmp = b'0' + (num % 10) as u8;
            }
            num /= 10;
            j += 1;
        }
        for k in (0..j).rev() {
            if i < buf.len() {
                if let Some(bf) = buf.get_mut(i) {
                    if let Some(tmp) = temp.get(k) {
                        *bf = *tmp
                    }
                }
                i += 1;
            }
        }
    }

    if i >= buf.len() {
        return "";
    }

    // Safety: we only use ASCII bytes, so this is always valid UTF-8.
    unsafe { core::str::from_utf8_unchecked(&buf[..i]) }
}

