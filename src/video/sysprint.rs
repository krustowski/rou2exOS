//System prints such as warnings etc
use crate::video::{vga, vga::Color as Color};
use spin::Mutex;

pub static SYSBUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());

const MAX_MSG_LEN: usize = 60;

#[derive(PartialEq, Copy, Clone)]
pub enum Result {
    Unknown,
    Passed,
    Failed,
    Skipped,
}

//Here for future proofing, if needed to call from another caller outside of sysbuffer
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



//TODO: add safety check so self.pos doesnt go out of bounds
	pub fn format(&mut self, msg: &'static str, res: Result) {
		if msg.len() <= MAX_MSG_LEN {
			self.append(msg.as_bytes());
			//9 is size of the brackets
			while self.pos <= vga::BUFFER_WIDTH - 9 {
				self.append(b".");
		
			}
			self.append(b"["); 
			self.flush(None);
			
			self.append(res.format().0);

			self.flush(Some(res.format().1));

			self.append(b"]");
			self.flush(None); 


	}
	}

    pub fn append(&mut self, s: &[u8]) {


        let len = s.len().min(BUFFER_SIZE - self.pos);

		self.buf[self.pos..self.pos + len].copy_from_slice(&s);

		self.pos += len;

		
	}


    /// Puts the contents of buf into the printb! or printb_color! macro.
    pub fn flush(&mut self, c: Option<Color>) {
		match c {
			Some(Color::Cyan) => {
			if let Some(buf) = self.buf.get(..self.pos) {
				printb_color!(buf, Cyan);
				self.pos = 0;
			}
			}
			Some(Color::Green) => {
			if let Some(buf) = self.buf.get(..self.pos) {
				printb_color!(buf, Green);
				self.pos = 0;

			}
			}
			Some(Color::Red) => {
			if let Some(buf) = self.buf.get(..self.pos) {
				printb_color!(buf, Red);
				self.pos = 0;

			}
			}
 			Some(Color::Yellow) => {
			if let Some(buf) = self.buf.get(..self.pos) {
				printb_color!(buf, Yellow);
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


