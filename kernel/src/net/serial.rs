use crate::input::port;
//use x86_64;

const COM1: u16 = 0x3F8;

pub fn init() {
    port::write(COM1 + 1, 0x00); // Disable interrupts
    port::write(COM1 + 3, 0x80); // Enable DLAB
    port::write(COM1, 0x03); // Set divisor to 3 (38400 baud)
    port::write(COM1 + 1, 0x00); // High byte divisor
    port::write(COM1 + 3, 0x03); // 8 bits, no parity, one stop bit
    port::write(COM1 + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
    port::write(COM1 + 4, 0x0B); // IRQs enabled, RTS/DSR set
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


/*pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    pub const unsafe fn new(port: u16) -> SerialPort {
        let serial = SerialPort { port };
        serial
    }

    pub fn init(&self) {
        unsafe {
            use x86_64::instructions::port::Port;

            let mut port = Port::new(self.port + 1); // Interrupt enable
            port.write(0x00u8);

            let mut port = Port::new(self.port + 3); // Line control
            port.write(0x80u8); // Enable DLAB

            let mut port = Port::new(self.port + 0); // Divisor Latch Low
            port.write(0x03u8); // 38400 baud

            let mut port = Port::new(self.port + 1); // Divisor Latch High
            port.write(0x00u8);

            let mut port = Port::new(self.port + 3); // Line control
            port.write(0x03u8); // 8 bits, no parity, one stop bit

            let mut port = Port::new(self.port + 2); // FIFO control
            port.write(0xC7u8);

            let mut port = Port::new(self.port + 4); // Modem control
            port.write(0x0Bu8);
        }
    }

    pub fn write_byte(&self, byte: u8) {
        unsafe {
            use x86_64::instructions::port::Port;

            let mut data_port: Port<u8> = Port::new(self.port);
            let mut line_status_port: Port<u8> = Port::new(self.port + 5);

            // Wait until the Transmitter Holding Register is empty
            while (line_status_port.read() & 0x20) == 0 {}

            data_port.write(byte);
        }
    }

    pub fn write_bytes(&self, buf: &[u8]) {
        for &b in buf {
            self.write_byte(b);
        }
    }
}*/

