#[derive(Copy,Clone)]
pub enum VideoMode {
    Framebuffer {
        address: *mut u8,
        pitch: usize,
        width: usize,
        height: usize,
        bpp: u8,
    },
    TextMode,
}

static mut VIDEO_MODE: Option<VideoMode> = Some(VideoMode::TextMode);

pub fn init_video(fb: &crate::init::boot::FramebufferTag) {
    unsafe {
    VIDEO_MODE = Some(VideoMode::Framebuffer {
        address: fb.addr as *mut u8,
        pitch: fb.pitch as usize,
        width: fb.width as usize,
        height: fb.height as usize,
        bpp: fb.bpp,
    });
    }
}

pub fn get_video_mode() -> Option<VideoMode> {
    unsafe {
        return VIDEO_MODE;
    }
}

pub fn put_pixel(x: usize, y: usize, r: u8, g: u8, b: u8) {
    unsafe {
        match VIDEO_MODE {
            Some(VideoMode::Framebuffer {
                address,
                pitch,
                width,
                height,
                bpp,
            }) => {
                if x >= width || y >= height {
                    return;
                }

                let offset = y * pitch + x * (bpp as usize / 8);
                let ptr = address.add(offset);

                match bpp {
                    32 => {
                        let color = (r as u32) << 16 | (g as u32) << 8 | (b as u32);
                        *(ptr as *mut u32) = color;
                    }
                    16 => {
                        let r5 = (r >> 3) as u16;
                        let g6 = (g >> 2) as u16;
                        let b5 = (b >> 3) as u16;
                        let color = (r5 << 11) | (g6 << 5) | b5;
                        *(ptr as *mut u16) = color;
                    }
                    _ => {} // Unsupported
                }
            }
            Some(VideoMode::TextMode) => {
                // No-op for pixel drawing in text mode
            }
            _ => {}
        }
    }
}

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const VGA_WIDTH: usize = 80;

pub fn put_char(x: usize, y: usize, ch: u8, color: u8) {
    unsafe {
        match VIDEO_MODE {
            Some(VideoMode::TextMode) => {
                let offset = 2 * (y * VGA_WIDTH + x);
                *VGA_BUFFER.add(offset) = ch;
                *VGA_BUFFER.add(offset + 1) = color;
            }
            _ => {
                // TODO: Render bitmap font to framebuffer
            }
        }
    }
}

