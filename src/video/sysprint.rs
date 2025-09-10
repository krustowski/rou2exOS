//system prints such as warnings etc
use crate::video::{vga, bufmg};
use spin::Mutex;

pub static SysBuffer: Mutex<bufmg::Buffer> = Mutex::new(bufmg::Buffer::new());

const MAX_MSG_LEN: usize = 60;

/*pub fn format_result(message: &'static str, result: sysprint::Result) {
    let mut buf = bufmg::Buffer::new(); //new buffer instance, make the init initialize this instead?
    
    buf.append(message.as_bytes()); //append as bytes

    for _ in 0..MAX_MSG_LEN - message.len() {
        buf.append(b"."); //write, not going past max
    }

    buf.append(b" [");
    buf.append(result.format().0);
    buf.append(b"]\n");

	//in range ...
    if let Some(slice) = buf.buf.get(..buf.pos) {
        //
        INIT_BUFFER.lock().append(slice); //buffer lock?
    }
}*/


#[derive(PartialEq, Copy, Clone)] //make this global?? could be reused for a bunch of things 
pub enum Result {
    Unknown,
    Passed,
    Failed,
    Skipped,
}

//match
impl Result {
    pub fn format(&self) -> (&[u8; 6], vga::Color) {
        match self {
            Result::Unknown => 
                (b"UNKNWN", vga::Color::Cyan),
            Result::Passed => 
                (b"  OK  ", vga::Color::Green),
            Result::Failed => 
                (b" FAIL ", vga::Color::Red),
            Result::Skipped => 
                (b" SKIP ", vga::Color::Yellow),
        }
    }
}
//make the sys print call this
