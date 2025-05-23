pub struct Screen;

impl Screen {
    const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
    const WIDTH: usize = 80;
    const HEIGHT: usize = 25;

    pub fn write_char(x: usize, y: usize, chr: u8, attr: u8) {
        let offset = 2 * (y * Self::WIDTH + x);
        unsafe {
            core::ptr::write_volatile(Self::VGA_BUFFER.add(offset), chr);
            core::ptr::write_volatile(Self::VGA_BUFFER.add(offset + 1), attr);
        }
    }

    pub fn clear(attr: u8) {
        for y in 0..Self::HEIGHT {
            for x in 0..Self::WIDTH {
                Self::write_char(x, y, b' ', attr);
            }
        }
    }
}

