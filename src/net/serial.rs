use crate::input::port;
//use x86_64;

const COM1: u16 = 0x3F8;

pub fn init() {
    port::write(COM1 + 1, 0x00);    // Disable interrupts
    port::write(COM1 + 3, 0x80);    // Enable DLAB
    port::write(COM1, 0x03);        // Set divisor to 3 (38400 baud)
    port::write(COM1 + 1, 0x00);    // High byte divisor
    port::write(COM1 + 3, 0x03);    // 8 bits, no parity, one stop bit
    port::write(COM1 + 2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
    port::write(COM1 + 4, 0x0B);    // IRQs enabled, RTS/DSR set
}

/// Check if a byte is available from UART
pub fn ready() -> bool {
    (port::read(COM1 + 5) & 1) != 0
}

/// Read a byte from UART
pub fn read() -> u8 {
    port::read(COM1)
}

/// Write a byte to UART
pub fn write(b: u8) {
    while (port::read(COM1 + 5) & 0x20) == 0 {}
    port::write(COM1, b);
}

