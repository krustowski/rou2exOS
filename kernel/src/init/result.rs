use crate::vga;

pub enum InitResult {
    Unknown,
    Passed,
    Failed,
    Skipped,
}

impl InitResult {
    pub fn format(&self) -> (&[u8; 6], vga::buffer::Color) {
        match self {
            InitResult::Unknown => 
                (b"UNKNWN", vga::buffer::Color::Cyan),
            InitResult::Passed => 
                (b"  OK  ", vga::buffer::Color::Green),
            InitResult::Failed => 
                (b" FAIL ", vga::buffer::Color::Red),
            InitResult::Skipped => 
                (b" SKIP ", vga::buffer::Color::Yellow),
        }
    }
}

const MAX_MSG_LEN: usize = 60;

pub fn print_result(message: &'static str, result: InitResult, vga_index: &mut isize) {
    vga::write::string(vga_index, message.as_bytes(), vga::buffer::Color::White);

    for i in 0..MAX_MSG_LEN - message.len() {
        vga::write::byte(vga_index, b'.', vga::buffer::Color::White);
    }

    vga::write::string(vga_index, b" [", vga::buffer::Color::White);
    vga::write::string(vga_index, result.format().0, result.format().1);
    vga::write::string(vga_index, b"]", vga::buffer::Color::White);
    vga::write::newline(vga_index);
}
