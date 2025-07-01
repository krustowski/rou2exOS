pub fn draw_char(c: u8, x: usize, y: usize, fb: *mut u32, pitch: usize, fg: u32, font: &[u8]) {
    let char_size = font[3] as usize;
    //let glyph = &font[4 + (c as usize * char_size)..];

    if let Some(glyph) = font.get(4 + (c as usize * char_size)..) {
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
}

//
//
//

pub fn print_result() -> super::result::InitResult {
    super::result::InitResult::Unknown
}

//
//
//

pub static PSF_FONT: &[u8] = include_bytes!("../../terminus-font.psf");

pub struct PsfFont<'a> {
    glyphs: &'a [u8],
    bytes_per_glyph: usize,
    height: usize,
    width: usize,
}

pub fn parse_psf(psf: &[u8]) -> Option<PsfFont> {
    if psf.starts_with(&[0x36, 0x04]) { // PSF1
        let glyph_size = psf[3] as usize;
        //let num_glyphs = if psf[2] & 0x01 != 0 { 512 } else { 256 };

        Some(PsfFont {
            glyphs: &psf[4..],
            bytes_per_glyph: glyph_size,
            height: glyph_size,
            width: 8,
        })
    } else if psf.starts_with(&[0x72, 0xb5, 0x4a, 0x86]) { // PSF2
        let header_len = u32::from_le_bytes(psf[8..12].try_into().unwrap()) as usize;
        let glyph_size = u32::from_le_bytes(psf[20..24].try_into().unwrap()) as usize;
        let height = u32::from_le_bytes(psf[24..28].try_into().unwrap()) as usize;
        let width = u32::from_le_bytes(psf[28..32].try_into().unwrap()) as usize;

        Some(PsfFont {
            glyphs: &psf[header_len..],
            bytes_per_glyph: glyph_size,
            height,
            width,
        })
    } else {
        None
    }
}

fn draw_char_psf(font: &PsfFont, ch: u8, x: usize, y: usize, color: u32, framebuffer: *mut u32, pitch: usize, bpp: usize) {
    let glyph_start = ch as usize * font.bytes_per_glyph;
    //let glyph = &font.glyphs[glyph_start..glyph_start + font.bytes_per_glyph];

    if let Some(glyph) = font.glyphs.get(glyph_start..glyph_start + font.bytes_per_glyph) {
        for row in 0..font.height {
            let row_byte = glyph[row];
            for col in 0..font.width {
                if (row_byte >> (7 - col)) & 1 != 0 {
                    unsafe { 
                        let offset = (y + row) * 4096 / 4 + (x + col);

                        framebuffer.add(offset as usize + 1).write_volatile(color);
                        //framebuffer.add(offset as usize + 1).write_volatile(0xfefab0);
                        //framebuffer.add(offset as usize + 2).write_volatile(0xdeadbeef);
                    }
                }
            }
        }
    }
}

pub fn draw_text_psf(text: &str, font: &PsfFont, x: usize, y: usize, color: u32, framebuffer: *mut u32, pitch: usize, bpp: usize) {
    let mut cx = x;

    for ch in text.bytes() {
        draw_char_psf(font, ch, cx, y, color, framebuffer, pitch, bpp);
        cx += font.width;
    }
}

