use core::fmt::{self, Write};
use core::ptr::Unique;

use crate::app::editor::{MAX_LINES, MAX_LINE_LEN};

/// VGA text mode buffer dimensions
const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

/// VGA color attributes
#[allow(dead_code)]
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    White = 15,
}

#[derive(Copy, Clone)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(fg: Color, bg: Color) -> Self {
        Self((bg as u8) << 4 | (fg as u8))
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    col_pos: usize,
    row_pos: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

// Implement core::fmt::Write so we can use `write!()`
impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

impl Writer {
    pub fn new() -> Self {
        Writer {
            col_pos: 0,
            row_pos: 0,
            color_code: ColorCode::new(Color::White, Color::Black),
            buffer: unsafe { Unique::new_unchecked(0xb8000 as *mut _) },
        }
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col_pos >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = self.row_pos;
                let col = self.col_pos;

                if row >= BUFFER_HEIGHT || col >= BUFFER_WIDTH {
                    return;
                }

                let color_code = self.color_code;
                let buf = self.buffer_mut();

                if let Some(row_buf) = buf.chars.get_mut(row) {
                    if let Some(cell) = row_buf.get_mut(col) {
                        *cell = ScreenChar {
                            ascii_character: byte,
                            color_code,
                        };
                        self.col_pos += 1;
                    }
                }
            }
        }
    }

    fn new_line(&mut self) {
        if self.row_pos < BUFFER_HEIGHT - 1 {
            self.row_pos += 1;
        } else {
            // scroll up
            for row in 1..BUFFER_HEIGHT {
                let buffer = self.buffer_mut();
                for col in 0..BUFFER_WIDTH {
                    buffer.chars[row - 1][col] = buffer.chars[row][col];
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
        }
        self.col_pos = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        let buffer = self.buffer_mut();
        for col in 0..BUFFER_WIDTH {
            buffer.chars[row][col] = blank;
        }
    }

    pub fn write_str_raw(&mut self, s: &str) {
        for &byte in s.as_bytes() {
            self.write_byte(byte);
        }
    }
}

