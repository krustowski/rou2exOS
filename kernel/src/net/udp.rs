#[repr(C, packed)]
pub struct UdpHeader {
    pub source_port: u16,
    pub dest_port: u16,
    pub length: u16,
    pub checksum: u16,
}

pub fn create_packet(
    _source_ip: [u8; 4],
    _dest_ip: [u8; 4],
    source_port: u16,
    dest_port: u16,
    payload: &[u8],
    out_buffer: &mut [u8],
) -> usize {
    let udp_len = 8 + payload.len(); // 8 bytes header + payload

    let header = UdpHeader {
        source_port: source_port.to_be(),
        dest_port: dest_port.to_be(),
        length: (udp_len as u16).to_be(),
        checksum: 0, // temporary 0, we'll compute later
    };

    // Copy header
    unsafe {
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<UdpHeader>(),
        );
        out_buffer[..header_bytes.len()].copy_from_slice(header_bytes);
    }

    // Copy payload
    out_buffer[8..8 + payload.len()].copy_from_slice(payload);

    // Calculate checksum (optional in UDP, but some OSes expect it!)
    // For now: leave checksum 0.

    udp_len
}

pub fn parse_packet(packet: &[u8]) -> Option<(u16, u16, &[u8])> {
    if packet.len() < 8 {
        return None;
    }

    let source_port = u16::from_be_bytes([packet[0], packet[1]]);
    let dest_port = u16::from_be_bytes([packet[2], packet[3]]);
    let length = u16::from_be_bytes([packet[4], packet[5]]);
    let _checksum = u16::from_be_bytes([packet[6], packet[7]]);

    if packet.len() < length as usize {
        return None;
    }

    let payload = &packet[8..length as usize];
    Some((source_port, dest_port, payload))
}

/// Calculate UDP checksum including IPv4 pseudo-header
pub fn get_checksum(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    udp_packet: &[u8], // whole UDP header + data
) -> u16 {
    let mut sum = 0u32;

    // --- Pseudo-header ---
    sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
    sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;
    sum += 0x11u8 as u32; // Protocol (UDP = 17 decimal)
    sum += udp_packet.len() as u32; // UDP length

    // --- UDP header + payload ---
    let mut i = 0;
    while i + 1 < udp_packet.len() {
        let word = u16::from_be_bytes([udp_packet[i], udp_packet[i + 1]]);
        sum = sum.wrapping_add(word as u32);
        i += 2;
    }

    if i < udp_packet.len() {
        let word = (udp_packet[i] as u16) << 8; // pad last byte
        sum = sum.wrapping_add(word as u32);
    }

    // Fold carries
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

