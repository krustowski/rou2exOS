use crate::input::port;
use crate::net::pci;

pub const PCI_VENDOR_ID_REALTEK: u16 = 0x10EC;
pub const PCI_DEVICE_ID_RTL8139: u16 = 0x8139;

pub static mut RTL8139_IO_BASE: u16 = 0xC000; // overwritten by rtl8139_init() from PCI BAR0
const NUM_TX_BUFFERS: usize = 4;

static mut RX_BUFFER: [u8; 8192 + 16 + 1500] = [0; 8192 + 16 + 1500];
static mut RX_OFFSET: usize = 0;

//#[repr(align(4))]
static mut TX_BUFFERS: [[u8; 2048]; NUM_TX_BUFFERS] = [[0; 2048]; NUM_TX_BUFFERS];
static mut TX_INDEX: usize = 0;

pub fn receive_frame(buf: &mut [u8]) -> Option<usize> {
    unsafe {
        // Check buffer-not-empty: CBR (current buffer read ptr, 0x3A) != CAPR+16 (0x38)
        let capr = port::read_u16(RTL8139_IO_BASE + 0x38).wrapping_add(16) as usize;
        let cbr = port::read_u16(RTL8139_IO_BASE + 0x3A) as usize;
        if capr == cbr {
            return None; // RX buffer empty
        }

        // Acknowledge any pending ROK in ISR
        port::write_u8(RTL8139_IO_BASE + 0x3E, 0x01);

        let offset = RX_OFFSET & 0x1FFF;
        //let rx_buf = &RX_BUFFER[offset..];

        #[expect(static_mut_refs)]
        if let Some(rx_buf) = RX_BUFFER.get(offset..) {
            if rx_buf.len() < 4 {
                return None;
            }

            let _rx_status = u16::from_le_bytes([rx_buf[0], rx_buf[1]]);
            let len = u16::from_le_bytes([rx_buf[2], rx_buf[3]]) as usize;

            if len < 14 || len > buf.len() {
                return None;
            }

            if let Some(bf) = buf.get_mut(..len) {
                if let Some(rx) = rx_buf.get(4..4 + len) {
                    bf.copy_from_slice(rx);
                }
            }
            //buf[..len].copy_from_slice(&rx_buf[4..4 + len]);

            RX_OFFSET = (RX_OFFSET + len + 4 + 3) & !3; // Align to 4 bytes

            // CAPR (0x38): tell the card how far we've read.
            // RTL8139 datasheet quirk: write (offset - 16) to avoid off-by-one
            // in the NIC's empty/full detection.
            port::write_u16(RTL8139_IO_BASE + 0x38, RX_OFFSET.wrapping_sub(16) as u16);

            Some(len)
        } else {
            None
        }
    }
}

/// Send `len` bytes from `data`. Ethernet minimum is 60 bytes; pad with zeros if shorter.
pub fn send_frame(data: &[u8], len: usize) -> Result<(), &'static str> {
    const ETH_MIN: usize = 60;
    let send_len = if len < ETH_MIN { ETH_MIN } else { len };

    if send_len > 2048 {
        return Err("Frame too large");
    }

    unsafe {
        let tx_idx = TX_INDEX;
        let buf = &mut TX_BUFFERS[tx_idx];
        buf[..len].copy_from_slice(&data[..len]);
        if len < ETH_MIN {
            // zero-pad to minimum frame size
            for b in buf[len..ETH_MIN].iter_mut() {
                *b = 0;
            }
        }

        let buf_phys = buf.as_ptr() as u32;
        let tx_addr_port = RTL8139_IO_BASE + 0x20 + (tx_idx * 4) as u16;
        port::write_u32(tx_addr_port, buf_phys);

        let tx_status_port = RTL8139_IO_BASE + 0x10 + (tx_idx * 4) as u16;
        port::write_u32(tx_status_port, send_len as u32);

        TX_INDEX = (TX_INDEX + 1) % NUM_TX_BUFFERS;
    }

    Ok(())
}


/// Read the 6-byte MAC address from RTL8139 IDR0-IDR5 registers.
/// Safe to call any time after rtl8139_init() has set RTL8139_IO_BASE.
pub unsafe fn read_mac_addr() -> [u8; 6] {
    let io = RTL8139_IO_BASE;
    [
        port::read_u8(io),     port::read_u8(io + 1),
        port::read_u8(io + 2), port::read_u8(io + 3),
        port::read_u8(io + 4), port::read_u8(io + 5),
    ]
}

pub fn rtl8139_init() {
    // Discover the actual I/O base from PCI BAR0 before touching any registers
    let discovered = pci::find_io_base(PCI_VENDOR_ID_REALTEK, PCI_DEVICE_ID_RTL8139);
    unsafe {
        if discovered != 0 {
            RTL8139_IO_BASE = discovered;
        }
    }

    pci::enable_bus_mastering(PCI_VENDOR_ID_REALTEK, PCI_DEVICE_ID_RTL8139);

    let io_base = unsafe { RTL8139_IO_BASE };

    // Reset ring-buffer read pointer so re-launch starts clean
    unsafe { RX_OFFSET = 0; }

    // Power on (CONFIG1 = 0x00 puts chip in normal power mode)
    port::write_u8(io_base + 0x52, 0x00);

    // Reset
    port::write_u8(io_base + 0x37, 0x10);
    while port::read_u8(io_base + 0x37) & 0x10 != 0 {}

    // Set receive buffer address
    let rx_buf_addr = &raw const RX_BUFFER as u32;
    port::write_u32(io_base + 0x30, rx_buf_addr);

    // Enable RX and TX
    port::write_u8(io_base + 0x37, 0x0C);

    // Set receive config
    port::write_u32(io_base + 0x44, 0xf | (1 << 7)); // Accept broadcast | multicast | runt

    // Enable RX OK interrupts
    port::write_u16(io_base + 0x3C, 0x0005);
}

