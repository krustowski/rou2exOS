use core::fmt::{self, Write};
use spin::Mutex;
use crate::vga::{write::string, buffer::Color, screen};

const DEBUG_LOG_SIZE: usize = 8192;

pub struct DebugLog {
    buffer: [u8; DEBUG_LOG_SIZE],
    len: usize,
}

pub static DEBUG_LOG: Mutex<DebugLog> = Mutex::new(DebugLog::new());

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

        self.len += to_copy;
    }
}

impl Write for DebugLog {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.append(s.as_bytes());
        Ok(())
    }
}

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

#[macro_export]
macro_rules! kprint {
    ($buf:expr, $off:expr, $str:expr) => {
        let len = $buf.len();

        if *$off >= len || *$off + $str.len() >= len {
            return;
        }

        if let Some(slice) = $buf.get_mut(*$off..*$off + $str.len()) {
            slice.copy_from_slice($str);
            *$off += $str.len();
        }
    };
}

use crate::fs::fat12::{block::Floppy, fs::Fs};

pub fn dump_debug_log_to_file(vga_index: &mut isize) {
    let dbg = DEBUG_LOG.lock();

    let floppy = Floppy;

    // Dump log to display
    screen::clear(&mut 0);
    string(&mut 0, dbg.data(), Color::Yellow);

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            // Dump debug data into the DEBUG.TXT file in root directory
            fs.write_file(0, b"DEBUG   TXT", dbg.data(), vga_index);
        }
        Err(e) => {
            debugln!(e);
            string(vga_index, e.as_bytes(), Color::Red);
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

