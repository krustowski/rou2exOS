use core::fmt::{self, Write};
use spin::Mutex;
use x86_64::registers::debug;
use crate::vga::{write::string, buffer::Color, screen};

const DEBUG_LOG_SIZE: usize = 8192;

pub struct DebugLog {
    buffer: [u8; DEBUG_LOG_SIZE],
    len: usize,
}

impl DebugLog {
    pub const fn new() -> Self {
        Self {
            buffer: [0; DEBUG_LOG_SIZE],
            len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn data(&self) -> &[u8] {
        //&self.buffer[..self.len];
        if let Some(d) = self.buffer.get(..self.len) {
            return d;
        }

        &[]
    }

    pub fn append(&mut self, data: &[u8]) {
        let remaining = DEBUG_LOG_SIZE - self.len;
        let to_copy = core::cmp::min(data.len(), remaining);


        if let Some(slice) = self.buffer.get_mut(self.len..self.len + to_copy) {
            if let Some(data) = data.get(..to_copy) {
                slice.copy_from_slice(data);
            }
        }
        //self.buffer[self.len..self.len + to_copy].copy_from_slice(&data[..to_copy]);

        self.len += to_copy;
    }
}

impl Write for DebugLog {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.append(s.as_bytes());
        Ok(())
    }
}

pub static DEBUG_LOG: Mutex<DebugLog> = Mutex::new(DebugLog::new());

pub fn u64_to_dec_str(mut n: u64, buf: &mut [u8; 20]) -> &[u8] {
    if n == 0 {
        buf[0] = b'0';
        return &buf[..1];
    }
    let mut i = 20;
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}

#[macro_export]
macro_rules! debugn {
    ($n:expr) => {{
        if let Some(mut log) = $crate::debug::DEBUG_LOG.try_lock() {
            let mut buf = [0u8; 20];
            let s = $crate::debug::u64_to_dec_str($n as u64, &mut buf);
            log.append(s);
        }
    }};
}

#[macro_export]
macro_rules! debug {
    ($s:expr) => {{
        if let Some(mut log) = $crate::debug::DEBUG_LOG.try_lock() {
            // Only &[u8], *str and b"literal" 
            let bytes = ($s).as_ref();
            log.append(bytes);
        }
    }};
}

#[macro_export]
macro_rules! debugln {
    ($s:expr) => {{
        $crate::debug!($s);
        $crate::debug!("\n");
    }};
}

use crate::fs::fat12::{block::Floppy, fs::Fs};

pub fn dump_debug_log_to_file(vga_index: &mut isize) {
    string(vga_index, b"jezisi", Color::Cyan);

    let dbg = DEBUG_LOG.lock();

    string(vga_index, b"kriste", Color::Cyan);

    let floppy = Floppy;

    /*screen::clear(&mut 0);
      string(&mut 0, dbg.data(), Color::Yellow);

      return;*/

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            string(vga_index, b"dost ano", Color::Cyan);

            // Dump debug data into the DEBUG.TXT file in root directory
            fs.write_file(0, b"DEBUG   TXT", dbg.data(), vga_index);
        }
        Err(e) => {
            debugln!(e);

            // Dump logs right into the display
            screen::clear(&mut 0);
            string(&mut 0, dbg.data(), Color::Yellow);
        }
    }
}

fn print_stack_info() {
    let sp: usize;
    unsafe {
        core::arch::asm!("mov {}, rsp", out(reg) sp);

        debug!("Stack pointer: ");
        debugn!(sp as u64);
        debugln!("");
    }
}

