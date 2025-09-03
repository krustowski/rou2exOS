//system prints such as warnings etc

use crate::{vga};
pub struct SysBuffer {
    buf: [u8; 1024],
    pos: usize,
}
const MAX_MSG_LEN: usize = 60;

impl SysBuffer {
    //get buffer instance
    pub const fn new() -> Self {
        Self {
            buf: [0u8; BUFFER_SIZE], //array of u8 of 1024
            pos: 0,
        }
    }

	pub fn format(&mut self, message: &'static str) {
		let len = message.len();
		
		//move -> then add [1234]
		

}

    /// Adds given byte slice to the buffer at offset of self.pos.
    pub fn append(&mut self, s: &[u8]) {
        // Take the input length, or the offset
		//s.len gives length of whats to be written, min compares minimum of comparison of 
		//selfs buf len - position
        let len = s.len().min(self.buf.len() - self.pos);
		//get mut returns mutable reference of self pos + len
        if let Some(buf) = self.buf.get_mut(self.pos..self.pos + len) {
            if let Some(slice) = s.get(..len) {
                // Copy the slice into buffer at offset of self.pos
                buf.copy_from_slice(slice);
                self.pos += len;
            }
        }
    }

    /// Puts the contents of buf into the printb! macro.
    pub fn flush(&self) {
        if let Some(buf) = self.buf.get(..self.pos) {
            printb!(buf);
        }
    }
}

#[derive(PartialEq, Copy, Clone)] //make this global?? could be reused for a bunch of things 
pub enum Result {
    Unknown,
    Passed,
    Failed,
    Skipped,
}
const BUFFER_SIZE: usize = 1024;
//match
impl Result {
    pub fn format(&self) -> (&[u8; 6], Color) {
        match self {
            Result::Unknown => 
                (b"UNKNWN", Color::Cyan),
            Result::Passed => 
                (b"  OK  ", Color::Green),
            Result::Failed => 
                (b" FAIL ", Color::Red),
            Result::Skipped => 
                (b" SKIP ", Color::Yellow),
        }
    }
}
//make the sys print call this
