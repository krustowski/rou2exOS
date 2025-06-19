use crate::vga::{
    write::{byte, string, newline},
    buffer::Color,
};

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

pub fn print_result(message: &'static str, result: InitResult, vga_index: &mut isize) {
    string(vga_index, message.as_bytes(), Color::White);

    for _ in 0..MAX_MSG_LEN - message.len() {
        byte(vga_index, b'.', Color::White);
    }

    string(vga_index, b" [", Color::White);
    string(vga_index, result.format().0, result.format().1);
    string(vga_index, b"]", Color::White);
    newline(vga_index);
}
