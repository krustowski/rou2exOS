use core::fmt::{self, Write};
use spin::Mutex;

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
        &self.buffer[..self.len]
    }

    pub fn append(&mut self, data: &[u8]) {
        let remaining = DEBUG_LOG_SIZE - self.len;
        let to_copy = core::cmp::min(data.len(), remaining);
        self.buffer[self.len..self.len + to_copy].copy_from_slice(&data[..to_copy]);
        self.len += to_copy;
    }
}

impl Write for DebugLog {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.append(s.as_bytes());
        Ok(())
    }
}

use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEBUG_LOG: Mutex<DebugLog> = Mutex::new(DebugLog::new());
}

#[macro_export]
macro_rules! debugf {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut dbg = $crate::debug_log::DEBUG_LOG.lock();
        let _ = writeln!(dbg, $($arg)*);
    });
}

use crate::fs::fat12::{block::Floppy, fs::Fs};

pub fn dump_debug_log_to_file() {
    let dbg = DEBUG_LOG.lock();

    let mut floppy = Floppy;

    match Fs::new(&floppy, &mut 0) {
        Ok(fs) => {
            // Dump debug data into the DEBUG.TXT file in root directory
            fs.write_file(0, b"DEBUG   TXT", dbg.data(), &mut 0);
        }
        Err(e) => {}
    }
}


