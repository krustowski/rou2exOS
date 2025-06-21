use core::fmt::{self, Write};
use core::ptr::Unique;
use crate::input::port;
use spin::{mutex::Mutex};
use core::sync::atomic::{AtomicBool, Ordering};

/// VGA text mode buffer dimensions.
const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_ADDRESS: usize = 0xb8000;

/// Wrapped Writer instance guarded by Mutex.
pub static mut WRITER: Option<Mutex<Writer>> = None;

/// Helper static boolean to ensure that the global Writer instance is created just once.
static WRITER_INIT: AtomicBool = AtomicBool::new(false);

/// Initializes the unique Writer instance.
pub fn init_writer() {
    if WRITER_INIT.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        let writter = Writer::new();
        unsafe {
            WRITER = Some(Mutex::new(writter));
        }
    }
}

/// Returns a wrapped Writer instance guarded by Mutex in Option. Beware that this invocation locks
/// the Writer instance and all print macros therefore can fail silently.
pub fn get_writer() -> Option<spin::MutexGuard<'static, Writer>> {
    if WRITER_INIT.load(Ordering::Relaxed) {
        unsafe { WRITER.as_ref().map(|m| m.lock()) }
    } else {
        // Not initialized yet
        None
    }
}

/// VGA text mode colors (16 colors).
#[allow(dead_code)]
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    DarkBlue = 1,
    DarkGreen = 2,
    DarkCyan = 3,
    DarkRed = 4,
    DarkMagenta = 5,
    DarkYellow = 6,
    LightGrey = 7,
    //
    Grey = 8,
    Blue = 9,
    Green = 10,
    Cyan = 11,
    Red = 12,
    Magenta = 13,
    Yellow = 14,
    White = 15,
}

/// Structure to abstract and combine the foreground and background color usage.
#[derive(Copy, Clone)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Creates a new ColorCode instance to be used in text video mode implementations.
    fn new(fg: Color, bg: Color) -> Self {
        Self((bg as u8) << 4 | (fg as u8))
    }
}

/// Structure to abstract a single character on the VGA text mode screen.
#[derive(Copy, Clone)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

/// Buffer abstracts the whole VGA text mode screen with the 2D array to hold 80x25 ScreenChars.
#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// Writer encapsulates the VGA text mode abstractions with proper video operations as
/// implementations.
pub struct Writer {
    col_pos: usize,
    row_pos: usize,
    color_code: ColorCode,
    buffer: Unique<Buffer>,
}

impl Write for Writer {
    /// Writes an input string to VGA text mode screen.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = self.write_str_raw(s);
        Ok(())
    }
}

impl Writer {
    /// Initializes a new Writer instance and returns it right away.
    pub fn new() -> Self {
        Writer {
            col_pos: 0,
            row_pos: 0,
            color_code: ColorCode::new(Color::White, Color::Black),
            buffer: unsafe { Unique::new_unchecked(BUFFER_ADDRESS as *mut _) },
        }
    }

    /// Clears the screen with the current ColorCode.
    pub fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);

            self.col_pos = 0;
            self.row_pos = 0;

            self.move_cursor();
        }
    }
    
    /// Sets the specified ColorCode from provided foreground and background colors.
    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = ColorCode::new(fg, bg)
    }

    /// Meta function to support printing static strings.
    pub fn write_str_raw(&mut self, s: &str) {
        for &byte in s.as_bytes() {
            self.write_byte(byte);
        }
    }
    /// Write one (1) byte to the display.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => {
                // Backspace = move cursor back and overwrite the ScreenChar on that position
                let mut row = self.row_pos;
                let mut col = self.col_pos;

                // Decrement the row position if we hit the left boundary of screen
                if col == 0 {
                    row -= 1;
                    col = BUFFER_WIDTH;
                } else {
                    col -= 1;
                }

                let color_code = self.color_code;
                let buf = self.buffer_mut();

                if let Some(row_buf) = buf.chars.get_mut(row) {
                    if let Some(cell) = row_buf.get_mut(col) {
                        *cell = ScreenChar {
                            ascii_character: b' ',
                            color_code,
                        };
                        self.col_pos = col;
                    }
                }
            }
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

                // Write the character to screen
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
        self.move_cursor();
    }

    /// Move the hardware cursor to (row, col)
    fn move_cursor(&mut self) {
        let pos: u16 = (self.row_pos * BUFFER_WIDTH + self.col_pos) as u16;

        // Set high byte
        port::write(0x3D4, 0x0E);
        port::write(0x3D5, (pos >> 8) as u8);

        // Set low byte
        port::write(0x3D4, 0x0F);
        port::write(0x3D5, (pos & 0xFF) as u8);
    }


    /// Returns a mutable reference to the video buffer.
    fn buffer_mut(&mut self) -> &mut Buffer {
        unsafe { self.buffer.as_mut() }
    }

    /// Does the CRLF type of magic with the positional coordinates of a Writer instance.
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
        self.move_cursor();
    }

    /// Clears the whole text row with the current ColorCode.
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
}

