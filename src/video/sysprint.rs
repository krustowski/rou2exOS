//system prints such as warnings etc
use crate::video::{vga, vga::Color as Color};
use spin::Mutex;

pub static SysBuffer: Mutex<Buffer> = Mutex::new(Buffer::new());

const MAX_MSG_LEN: usize = 60;

#[derive(PartialEq, Copy, Clone)]
pub enum Result {
    Unknown,
    Passed,
    Failed,
    Skipped,
}

//here for future proofing, if needed to call from another caller outside of sys buffer
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

const BUFFER_SIZE: usize = 160;


pub struct Buffer {
    pub buf: [u8; 160],
    pub pos: usize,
}




impl Buffer {
    //get buffer instance
    pub const fn new() -> Self {
        Self {
            buf: [0u8; BUFFER_SIZE], 
            pos: 0,
        }

    }




	pub fn format(&mut self, msg: &'static str, res: Result) {
		if msg.len() <= MAX_MSG_LEN {

			self.append(msg.as_bytes());

			while self.pos <= vga::BUFFER_WIDTH - 9 {
				self.append(b".");
		
			}
			self.append(b"[       ]"); //make this aligned with a for loop or while
			self.pos -= 7;
			self.flush(None);
			self.append(res.format().0);

			self.flush(Some(res.format().1));


	}
	}

    pub fn append(&mut self, s: &[u8]) {


        let len = s.len().min(BUFFER_SIZE - self.pos);

		self.buf[self.pos..self.pos + len].copy_from_slice(&s);

		self.pos += len;

		
	}


    /// Puts the contents of buf into the printb! macro.
    pub fn flush(&mut self, c: Option<Color>) {
		match c {
			Some(Color::Cyan) => {
			if let Some(buf) = self.buf.get(..self.pos) {
				printb!(buf, Cyan);
				self.pos = 0;

			}
			}

			None => {
			if let Some(buf) = self.buf.get(..self.pos) {
            	printb!(buf);
				self.pos = 0;
        	}

			}

			_ => ()
		}

    }

}


