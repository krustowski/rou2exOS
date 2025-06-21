use crate::fs::fat12::{fs::Fs, block::Floppy};
use crate::init::config::PATH_CLUSTER;
use crate::input::keyboard::{self, keyboard_read_scancode};
use crate::vga::{buffer::Color, screen::clear, write::{string, newline}};
use crate::video::vga::Writer;

pub const MAX_LINES: usize = 100;
pub const MAX_LINE_LEN: usize = 80;

static mut SHIFT_PRESSED: bool = false;
static mut CAPS_LOCK_ON: bool = false;

pub struct Editor {
    pub buffer: [[u8; MAX_LINE_LEN]; MAX_LINES],
    pub line_count: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub insert_mode: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: [[b' '; MAX_LINE_LEN]; MAX_LINES],
            line_count: 1,
            cursor_x: 0,
            cursor_y: 0,
            insert_mode: false,
        }
    }

    pub fn load(&mut self, data: &[u8]) {
        let mut line = 0;
        let mut col = 0;

        for &byte in data {
            match byte {
                b'\n' | b'\r' => {
                    line += 1;
                    col = 0;
                    if line >= MAX_LINES {
                        break;
                    }
                }
                _ => {
                    if col < MAX_LINE_LEN && line < MAX_LINES {
                        self.buffer[line][col] = byte;
                        col += 1;
                    }
                }
            }
        }

        self.line_count = line + 1;
    }

    pub fn save(&self, out: &mut [u8]) -> usize {
        let mut idx = 0;

        for y in 0..self.line_count {
            if let Some(row) = self.buffer.get(y) {
            let last_non_space = row.iter().rposition(|&c| c != b' ');

            if let Some(last) = last_non_space {
                for &c in &row[..=last] {
                    if idx >= out.len() {
                        return idx;
                    }
                    if let Some(o) = out.get_mut(idx) {
                        *o = c;
                        idx += 1;
                    }
                }
            }

            if idx < out.len() {
                if let Some(o) = out.get_mut(idx) {
                    *o = b'\n';
                    idx += 1;
                }
            } else {
                break;
            }
        }
        }

        idx
    }

    pub fn render(&self, writer: &mut Writer, filename: &[u8; 12]) {
        for y in 0..self.line_count {
            for x in 0..MAX_LINE_LEN {
                if let Some(rw) = self.buffer.get(y) {
                    if let Some(c) = rw.get(x) {
                        writer.write_byte(*c);
                    }
                }
            }
            writer.write_byte(b'\n');
        }

        // Status bar
        writer.write_byte(b'\n');

        writer.write_str_raw("MODE: ");
        if self.insert_mode {
            writer.write_str_raw("INSERT");
        } else {
            writer.write_str_raw("OVERWRITE");
        }

        writer.write_str_raw(" | CURSOR: ");
        writer.write_byte(b'0' + (self.cursor_y as u8 / 10));
        writer.write_byte(b'0' + (self.cursor_y as u8 % 10));
        writer.write_byte(b':');
        writer.write_byte(b'0' + (self.cursor_x as u8 / 10));
        writer.write_byte(b'0' + (self.cursor_x as u8 % 10));

        writer.write_str_raw(" | Ctrl+S=Save  Ctrl+Q=Quit | ");

        for &b in &filename[0..8] {
            if b == b' ' {
                break;
            }
            writer.write_byte(b);
        }

        writer.write_byte(b'.');

        for &b in &filename[8..11] {
            if b == b' ' {
                break;
            }
            writer.write_byte(b);
        }

    }

    pub fn handle_key(&mut self, key: u8) {
        match key {
            b'\n' | b'\r' => {
                if self.cursor_y + 1 < MAX_LINES {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                    if self.cursor_y >= self.line_count {
                        self.line_count = self.cursor_y + 1;
                    }
                }
            }
            8 => { // Backspace
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;

                    if let Some(y) = self.buffer.get_mut(self.cursor_y) {
                        if let Some(c) = y.get_mut(self.cursor_x) {
                            *c = b' ';
                        }
                    }

                    //self.buffer[self.cursor_y][self.cursor_x] = b' ';
                }
            }
            b => {
                if self.cursor_y < self.buffer.len() && self.cursor_x < MAX_LINE_LEN {
                    if let Some(line) = self.buffer.get_mut(self.cursor_y) {
                        if self.insert_mode {
                            // Shift characters right
                            for i in (self.cursor_x + 1..MAX_LINE_LEN).rev() {
                                let mut prev = [0u8; 80];
                                prev.copy_from_slice(line);

                                if i >= MAX_LINE_LEN || i < 1 {
                                    return;
                                }

                                line[i] = line[i - 1];
                            }
                        }

                        if let Some(cur) = line.get_mut(self.cursor_x) {
                            *cur = b;
                        }

                        if self.cursor_x + 1 < MAX_LINE_LEN {
                            self.cursor_x += 1;
                        }
                    }
                }
            }
        }
    }
}

pub fn edit_file(file_name: &[u8; 12], vga_index: &mut isize) {
    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Err(e) => {
            string(vga_index, e.as_bytes(), Color::Red);
            newline(vga_index);
            return;
        }
        Ok(fs) => {
            let mut file_buf = [0u8; 512];

            let (name, ext) = split_filename(file_name);

            let mut filename = [0u8; 11];
            if let Some(slice) = filename.get_mut(..name.len()) {
                slice.copy_from_slice(name);
            }

            let mut cluster: u16 = 0;

            unsafe {
                fs.for_each_entry(PATH_CLUSTER, |entry| {
                    if entry.name[0] != 0x00 && entry.name[0] != 0xE5 && entry.attr & 0x10 == 0 {

                        if entry.name.starts_with(name) {
                            cluster = entry.start_cluster;
                            return;
                        }
                    }
                }, &mut 0);

                if cluster <= 0 {
                    return;
                }
                fs.read_file(cluster, &mut file_buf, vga_index);
            }

            let mut editor = Editor::new();
            editor.load(&file_buf);

            let mut ctrl_down = false;

            loop {
                clear(vga_index);
                let mut wr = Writer::new(); // RECREATE each frame
                editor.render(&mut wr, file_name);

                // Set cursor position on screen
                keyboard::move_cursor(editor.cursor_y as u16, editor.cursor_x as u16);

                let key = keyboard_read_scancode();

                if key & 0x80 != 0 {
                    // Key released
                    let released = key & 0x7F;
                    if released == 0x1D {
                        ctrl_down = false;
                    }
                    continue;
                }

                match key {
                    // ESC
                    0x01 => break,
                    0x1D => {
                        ctrl_down = true;
                    }
                    0x1F => {
                        if ctrl_down {
                            let mut save_buf = [0u8; 4096];
                            let len = editor.save(&mut save_buf);

                            wr.write_str_raw("SAVING...");

                            unsafe {
                                if let Some(slice) = save_buf.get(..len) { 
                                    let mut filename = [b' '; 11];
                                    if name.len() <= 8 && ext.len() <= 3 {
                                        filename[..name.len()].copy_from_slice(name);
                                        filename[8..8 + ext.len()].copy_from_slice(ext);

                                        fs.write_file(PATH_CLUSTER, &filename, slice, vga_index);
                                    }
                                }
                            }
                        } else {
                            editor.handle_key(b's');
                        }
                    }
                    0x52 => {
                        editor.insert_mode = !editor.insert_mode;
                    }
                    0x4B => { // Left Arrow
                        if editor.cursor_x > 0 {
                            editor.cursor_x -= 1;
                        }
                    }
                    0x4D => { // Right Arrow
                        if editor.cursor_x + 1 < MAX_LINE_LEN {
                            editor.cursor_x += 1;
                        }
                    }
                    0x48 => { // Up Arrow
                        if editor.cursor_y > 0 {
                            editor.cursor_y -= 1;
                        }
                    }
                    0x50 => { // Down Arrow
                        if editor.cursor_y + 1 < editor.line_count {
                            editor.cursor_y += 1;
                        }
                    }
                    byte => {
                        if let Some(ascii) = scancode_to_ascii(byte) {
                            editor.handle_key(ascii);
                        }
                    }
                }
            }
        }
    }
}

pub fn scancode_to_ascii(sc: u8) -> Option<u8> {
    unsafe {
        match sc {
            // Modifier keys
            0x2A | 0x36 => {
                SHIFT_PRESSED = true;
                return None;
            }
            0xAA | 0xB6 => {
                SHIFT_PRESSED = false;
                return None;
            }
            0x3A => {
                CAPS_LOCK_ON = !CAPS_LOCK_ON;
                return None;
            }

            // Printable keys
            _ => {}
        }

        let shifted = SHIFT_PRESSED;
        let caps = CAPS_LOCK_ON;

        let ch = match sc {
            // Number row (with Shift symbols)
            0x02 => if shifted { b'!' } else { b'1' },
            0x03 => if shifted { b'@' } else { b'2' },
            0x04 => if shifted { b'#' } else { b'3' },
            0x05 => if shifted { b'$' } else { b'4' },
            0x06 => if shifted { b'%' } else { b'5' },
            0x07 => if shifted { b'^' } else { b'6' },
            0x08 => if shifted { b'&' } else { b'7' },
            0x09 => if shifted { b'*' } else { b'8' },
            0x0A => if shifted { b'(' } else { b'9' },
            0x0B => if shifted { b')' } else { b'0' },
            0x0C => if shifted { b'_' } else { b'-' },
            0x0D => if shifted { b'+' } else { b'=' },

            // Letters (Caps Lock + Shift logic)
            0x10..=0x19 | 0x1E..=0x26 | 0x2C..=0x32 => {
                let lower = match sc {
                    0x10 => b'q', 0x11 => b'w', 0x12 => b'e', 0x13 => b'r', 0x14 => b't',
                    0x15 => b'y', 0x16 => b'u', 0x17 => b'i', 0x18 => b'o', 0x19 => b'p',
                    0x1E => b'a', 0x1F => b's', 0x20 => b'd', 0x21 => b'f', 0x22 => b'g',
                    0x23 => b'h', 0x24 => b'j', 0x25 => b'k', 0x26 => b'l',
                    0x2C => b'z', 0x2D => b'x', 0x2E => b'c', 0x2F => b'v',
                    0x30 => b'b', 0x31 => b'n', 0x32 => b'm',
                    _ => return None,
                };
                let upper = lower.to_ascii_uppercase();
                if caps ^ shifted { upper } else { lower }
            }

            // Punctuation
            0x1A => if shifted { b'{' } else { b'[' },
            0x1B => if shifted { b'}' } else { b']' },
            0x27 => if shifted { b':' } else { b';' },
            0x28 => if shifted { b'"' } else { b'\'' },
            0x29 => if shifted { b'~' } else { b'`' },
            0x2B => if shifted { b'|' } else { b'\\' },
            0x33 => if shifted { b'<' } else { b',' },
            0x34 => if shifted { b'>' } else { b'.' },
            0x35 => if shifted { b'?' } else { b'/' },

            // Control keys
            0x0E => 8,         // Backspace
            0x1C => b'\n',     // Enter
            0x39 => b' ',      // Space

            _ => return None,
        };

        Some(ch)
    }
}

pub fn split_filename(input: &[u8]) -> (&[u8], &[u8]) {
    let len = input.iter().position(|&c| c == 0).unwrap_or(input.len());
    let trimmed = &input[..len];

    if let Some(pos) = trimmed.iter().position(|&c| c == b'.') {
        let cmd = &trimmed[..pos];
        let mut rest = &trimmed[pos + 1..];
        while rest.first() == Some(&b' ') {
            rest = &rest[1..];
        }
        (cmd, rest)
    } else {
        (&trimmed[..], &[])
    }
}

