use crate::{init::Buffer, video::vga::Color};

use super::INIT_BUFFER;

#[derive(PartialEq, Copy, Clone)]
pub enum InitResult {
    Unknown,
    Passed,
    Failed,
    Skipped,
}

impl InitResult {
    pub fn format(&self) -> (&[u8; 6], Color) {
        match self {
            InitResult::Unknown => 
                (b"UNKNWN", Color::Cyan),
            InitResult::Passed => 
                (b"  OK  ", Color::Green),
            InitResult::Failed => 
                (b" FAIL ", Color::Red),
            InitResult::Skipped => 
                (b" SKIP ", Color::Yellow),
        }
    }
}

const MAX_MSG_LEN: usize = 60;

pub fn print_result(message: &'static str, result: InitResult) {
    let mut buf = Buffer::new();
    
    buf.append(message.as_bytes());

    for _ in 0..MAX_MSG_LEN - message.len() {
        buf.append(b".");
    }

    buf.append(b" [");
    buf.append(result.format().0);
    buf.append(b"]\n");

    if let Some(slice) = buf.buf.get(..buf.pos) {
        //
        INIT_BUFFER.lock().append(slice);
    }
}
