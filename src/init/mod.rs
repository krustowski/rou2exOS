pub mod ascii;
pub mod boot;
pub mod color;
pub mod config;
pub mod cpu;
pub mod fs;
pub mod heap;
pub mod result;
pub mod vga;

use spin::Mutex;

const BUFFER_SIZE: usize = 1024;

static INIT_BUFFER: Mutex<Buffer> = Mutex::new(Buffer::new());

struct Buffer {
    buf: [u8; 1024],
    pos: usize,
}

impl Buffer {
    /// Creates and returns a new instance of Buffer.
    const fn new() -> Self {
        Self {
            buf: [0u8; BUFFER_SIZE],
            pos: 0,
        }
    }

    /// Adds given byte slice to the buffer at offset of self.pos.
    fn append(&mut self, s: &[u8]) {
        // Take the input length, or the offset
        let len = s.len().min(self.buf.len() - self.pos);

        if let Some(buf) = self.buf.get_mut(self.pos..self.pos + len) {
            if let Some(slice) = s.get(..len) {
                // Copy the slice into buffer at offset of self.pos
                buf.copy_from_slice(slice);
                self.pos += len;
            }
        }
    }

    /// Puts the contents of buf into the printb! macro.
    fn flush(&self) {
        if let Some(buf) = self.buf.get(..self.pos) {
            printb!(buf);
        }
    }
}

pub fn init(vga_index: &mut isize, multiboot_ptr: u64) {
    debugln!("Kernel init start");

    result::print_result(
        "Load kernel", 
        result::InitResult::Passed,
    );

    result::print_result(
        "Check 64-bit Long Mode", 
        cpu::check_mode(),
    );

    result::print_result(
        "Initialize heap allocator", 
        heap::print_result(vga_index),
    );

    result::print_result(
        "Read Multiboot2 tags", 
        boot::print_info(vga_index, multiboot_ptr),
    );

    let video_result = vga::print_result();

    result::print_result(
        "Initialize VGA writer", 
        *&video_result,
    );

    result::print_result(
        "Check floppy drive", 
        fs::check_floppy(vga_index),
    );

    // TODO: Fallback to floppy to dump debug logs + init buffer
    if video_result == result::InitResult::Passed {
        INIT_BUFFER.lock().flush();
    }

    color::color_demo();
    ascii::ascii_art();

    // Play startup melody
    crate::audio::midi::play_melody();
}
