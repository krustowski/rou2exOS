#[repr(C)]
pub struct PSF1Header {
    magic: [u8; 2], 
    mode: u8,
    charsize: u8,
}

pub static FONT_RAW: &[u8] = include_bytes!("../../font.psf");

pub fn draw_char(c: u8, x: usize, y: usize, fb: *mut u8, pitch: usize, fg: u32, font: &[u8]) {
    let char_size = font[3] as usize;
    let glyph = &font[4 + (c as usize * char_size)..];

    for row in 0..char_size {
        let row_byte = glyph[row];
        for col in 0..8 {
            if (row_byte >> (7 - col)) & 1 != 0 {
                let px = x + col;
                let py = y + row;
                let offset = py * pitch + px * 4;
                unsafe {
                    let pixel_ptr = fb.add(offset) as *mut u32;
                    *pixel_ptr = fg;
                }
            }
        }
    }
}

pub fn draw_text(
    fb: &mut [u32],
    pitch: usize,
    width: usize,
    x: usize,
    y: usize,
    font: &[u8],
    text: &str,
    color: [u8; 4],
) {
    for (i, ch) in text.bytes().enumerate() {
        //draw_char(fb, pitch, width, x + i * 8, y, font, ch, color);
    }
}

pub fn print_result() -> super::result::InitResult {
    super::result::InitResult::Unknown
}
