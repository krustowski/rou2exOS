use crate::input::port;
use crate::net::pci;

pub const PCI_VENDOR_ID_REALTEK: u16 = 0x10EC;
pub const PCI_DEVICE_ID_RTL8139: u16 = 0x8139;

const RTL8139_IO_BASE: u16 = 0xC000;
const NUM_TX_BUFFERS: usize = 4;

static mut RX_BUFFER: [u8; 8192 + 16 + 1500] = [0; 8192 + 16 + 1500];
static mut RX_OFFSET: usize = 0;

//#[repr(align(4))]
static mut TX_BUFFERS: [[u8; 2048]; NUM_TX_BUFFERS] = [[0; 2048]; NUM_TX_BUFFERS];
static mut TX_INDEX: usize = 0;

pub fn receive_frame(buf: &mut [u8]) -> Option<usize> {
    unsafe {
        let isr = port::read(RTL8139_IO_BASE + 0x3E); // ISR (Interrupt Status Register)
        if isr & 0x01 == 0 {
            return None; // No packet received
        }

        port::write_u8(RTL8139_IO_BASE + 0x3E, 0x01); // Acknowledge RX interrupt

        let offset = RX_OFFSET & 0x1FFF;
        //let rx_buf = &RX_BUFFER[offset..];

        if let Some(rx_buf) = RX_BUFFER.get(offset..) {
            if rx_buf.len() < 4 {
                return None;
            }

            let rx_status = u16::from_le_bytes([rx_buf[0], rx_buf[1]]);
            let len = u16::from_le_bytes([rx_buf[2], rx_buf[3]]) as usize;

            if len == 0 || len > buf.len() {
                return None;
            }

            if let Some(bf) = buf.get_mut(..len) {
                if let Some(rx) = rx_buf.get(4..4 + len) {
                    bf.copy_from_slice(rx);
                }
            }
            //buf[..len].copy_from_slice(&rx_buf[4..4 + len]);

            RX_OFFSET = (RX_OFFSET + len + 4 + 3) & !3; // Align to 4 bytes

            // Tell the card the packet has been read
            port::write_u16(RTL8139_IO_BASE + 0x38, RX_OFFSET as u16);

            Some(len)
        } else {
            None
        }
    }
}

pub fn send_frame(data: &[u8]) -> Result<(), &'static str> {
    if data.len() > 2048 {
        return Err("Frame too large");
    }

    unsafe {
        let tx_idx = TX_INDEX;
        let buf = &mut TX_BUFFERS[tx_idx];
        buf[..data.len()].copy_from_slice(data);

        // Write buffer address
        let buf_phys = buf.as_ptr() as u32;
        let tx_addr_port = RTL8139_IO_BASE + 0x20 + (tx_idx * 4) as u16;
        port::write_u32(tx_addr_port, buf_phys);

        // Write length
        let tx_status_port = RTL8139_IO_BASE + 0x10 + (tx_idx * 4) as u16;
        port::write_u32(tx_status_port, data.len() as u32);

        // Advance TX index
        TX_INDEX = (TX_INDEX + 1) % NUM_TX_BUFFERS;
    }

    Ok(())
}


pub fn rtl8139_init() {
    unsafe {
        // Enable bus mastering
        pci::enable_bus_mastering(PCI_VENDOR_ID_REALTEK, PCI_DEVICE_ID_RTL8139);

        let io_base = RTL8139_IO_BASE;

        // Reset
        port::write_u8(io_base + 0x37, 0x10);
        while port::read_u8(io_base + 0x37) & 0x10 != 0 {}

        // Set receive buffer address
        let rx_buf_addr = &RX_BUFFER as *const _ as u32;
        port::write_u32(io_base + 0x30, rx_buf_addr);

        // Enable RX and TX
        port::write_u8(io_base + 0x37, 0x0C);

        // Set receive config
        port::write_u32(io_base + 0x44, 0xf | (1 << 7)); // Accept broadcast | multicast | runt

        // Enable RX OK interrupts
        port::write_u16(io_base + 0x3C, 0x0005);
    }
}

