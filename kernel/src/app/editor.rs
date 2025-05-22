use crate::fs::{fat12::Fs, block::Floppy};
use crate::init::config::PATH_CLUSTER;
use crate::input::keyboard::{self, keyboard_read_scancode};
use crate::vga::{buffer::Color, screen::clear, write::{string, newline}, writer::Writer};

pub const MAX_LINES: usize = 100;
pub const MAX_LINE_LEN: usize = 80;

static mut SHIFT_PRESSED: bool = false;
static mut CAPS_LOCK_ON: bool = false;

pub struct Editor {
    pub buffer: [[u8; MAX_LINE_LEN]; MAX_LINES],
    pub line_count: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: [[b' '; MAX_LINE_LEN]; MAX_LINES],
            line_count: 1,
            cursor_x: 0,
            cursor_y: 0,
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
            let mut last_non_space = row.iter().rposition(|&c| c != b' ');

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

    pub fn render(&self, writer: &mut Writer) {
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
                if self.cursor_y < MAX_LINES && self.cursor_x < MAX_LINE_LEN {

                    if let Some(y) = self.buffer.get_mut(self.cursor_y) {
                        if let Some(c) = y.get_mut(self.cursor_x) {
                            *c = b;
                        }
                    }
                    //self.buffer[self.cursor_y][self.cursor_x] = b;

                    self.cursor_x += 1;

                    if self.cursor_x >= MAX_LINE_LEN {
                        self.cursor_x = 0;
                        if self.cursor_y + 1 < MAX_LINES {
                            self.cursor_y += 1;
                            if self.cursor_y >= self.line_count {
                                self.line_count = self.cursor_y + 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn edit_file(file_name: &[u8; 11], vga_index: &mut isize) {
    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Err(e) => {
            string(vga_index, e.as_bytes(), Color::Red);
            newline(vga_index);
            return;
        }
        Ok(fs) => {
            let mut file_buf = [0u8; 512];

            unsafe {
                let cluster = fs.list_dir(PATH_CLUSTER, file_name, vga_index);
                if cluster <= 0 {
                    return;
                }
                fs.read_file(cluster as u16, &mut file_buf, vga_index);
            }

            let mut editor = Editor::new();
            editor.load(&file_buf);

            loop {
                clear(vga_index);
                let mut wr = Writer::new(); // RECREATE each frame
                editor.render(&mut wr);

                // Optional: set cursor position on screen
                keyboard::move_cursor(editor.cursor_y as u16, editor.cursor_x as u16);

                let key = keyboard_read_scancode();

                if key == 0x01 {
                    let mut save_buf = [0u8; 4096];
                    let len = editor.save(&mut save_buf);

                    unsafe {
                        if let Some(slice) = save_buf.get(..len) { 
                            fs.write_file(PATH_CLUSTER, file_name, slice, vga_index);
                        }
                    }
                    break;
                }

                if key & 0x80 == 0 {
                    // Key press
                    if let Some(ascii) = scancode_to_ascii(key) {
                        editor.handle_key(ascii);
                    }
                } else {
                    // Key release — for Shift handling
                    scancode_to_ascii(key); // To update SHIFT_PRESSED
                }
            }
        }
    }
}

/*pub fn scancode_to_ascii(sc: u8) -> Option<u8> {
  match sc {
  0x02 => Some(b'1'),
  0x03 => Some(b'2'),
  0x04 => Some(b'3'),
  0x05 => Some(b'4'),
  0x06 => Some(b'5'),
  0x07 => Some(b'6'),
  0x08 => Some(b'7'),
  0x09 => Some(b'8'),
  0x0A => Some(b'9'),
  0x0B => Some(b'0'),
  0x0C => Some(b'-'),
  0x0D => Some(b'='),
  0x0E => Some(8),    // Backspace
  0x0F => Some(b'\t'),
  0x10 => Some(b'q'),
  0x11 => Some(b'w'),
  0x12 => Some(b'e'),
  0x13 => Some(b'r'),
  0x14 => Some(b't'),
  0x15 => Some(b'y'),
  0x16 => Some(b'u'),
  0x17 => Some(b'i'),
  0x18 => Some(b'o'),
  0x19 => Some(b'p'),
  0x1A => Some(b'['),
  0x1B => Some(b']'),
  0x1C => Some(b'\n'), // Enter
  0x1E => Some(b'a'),
  0x1F => Some(b's'),
  0x20 => Some(b'd'),
  0x21 => Some(b'f'),
  0x22 => Some(b'g'),
  0x23 => Some(b'h'),
  0x24 => Some(b'j'),
  0x25 => Some(b'k'),
  0x26 => Some(b'l'),
  0x27 => Some(b';'),
  0x28 => Some(b'\''),
  0x29 => Some(b'`'),
  0x2C => Some(b'z'),
  0x2D => Some(b'x'),
  0x2E => Some(b'c'),
  0x2F => Some(b'v'),
  0x30 => Some(b'b'),
  0x31 => Some(b'n'),
  0x32 => Some(b'm'),
  0x33 => Some(b','),
  0x34 => Some(b'.'),
  0x35 => Some(b'/'),
  0x39 => Some(b' '),  // Spacebar
  _ => None,
  }
  }*/

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

