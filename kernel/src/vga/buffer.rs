pub const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;

pub const WIDTH: usize = 80;
pub const HEIGHT: usize = 25;

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Blue = 0x09,
    White = 0x0f,
    Green = 0xa,
    Cyan = 0xb,
    Red = 0xc,
    Pink = 0xd,
    Yellow = 0xe,
}
