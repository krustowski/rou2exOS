use crate::input::port;

fn pci_config_address(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset as u32) & 0xFC)
}

fn pci_config_read_u32(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = pci_config_address(bus, device, function, offset);
    port::write_u32(0xCF8, address);
    port::read_u32(0xCFC)
}

fn pci_config_read_u16(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let address = pci_config_address(bus, device, function, offset);
    port::write_u32(0xCF8, address);
    (port::read_u32(0xCFC) >> ((offset & 2) * 8)) as u16
}

fn pci_config_write_u16(bus: u8, device: u8, function: u8, offset: u8, value: u16) {
    let address = pci_config_address(bus, device, function, offset);
    port::write_u32(0xCF8, address);
    let old = port::read_u32(0xCFC);
    let shift = (offset & 2) * 8;
    let mask = !(0xFFFF << shift);
    let new = (old & mask) | ((value as u32) << shift);
    port::write_u32(0xCFC, new);
}

pub fn enable_bus_mastering(vendor_id: u16, device_id: u16) {
    for bus in 0..=255u8 {
        for device in 0..32u8 {
            for function in 0..8u8 {
                let id = pci_config_read_u32(bus, device, function, 0x00);
                if id == 0xFFFFFFFF {
                    continue;
                }

                let found_vendor = (id & 0xFFFF) as u16;
                let found_device = ((id >> 16) & 0xFFFF) as u16;

                if found_vendor == vendor_id && found_device == device_id {
                    let command = pci_config_read_u16(bus, device, function, 0x04);
                    pci_config_write_u16(bus, device, function, 0x04, command | 0x0004); // Set bus master bit
                    return;
                }
            }
        }
    }
}

