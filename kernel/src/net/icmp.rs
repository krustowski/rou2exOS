#[repr(C, packed)]
pub struct IcmpHeader {
    pub icmp_type: u8,
    pub icmp_code: u8,
    pub checksum: u16,
    pub identifier: u16,
    pub sequence_number: u16,
}

pub fn create_packet(
    packet_type: u8,           // 8 for Echo Request, 0 for Echo Reply
    identifier: u16,
    sequence_number: u16,
    payload: &[u8],
    out_buffer: &mut [u8],
) -> usize {
    let header_len = 8; // ICMP header is 8 bytes

    let header = IcmpHeader {
        icmp_type: packet_type,
        icmp_code: 0, // Usually 0
        checksum: 0,  // Fill later
        identifier,
        sequence_number,
    };

    // Copy header
    unsafe {
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<IcmpHeader>(),
        );
        out_buffer[..header_bytes.len()].copy_from_slice(header_bytes);
    }

    // Copy payload
    out_buffer[header_len..header_len + payload.len()].copy_from_slice(payload);

    // Calculate checksum (over full packet: header + payload)
    let checksum = get_checksum(&out_buffer[..header_len + payload.len()]);

    // Store checksum (Big Endian / Network order!)
    out_buffer[2..4].copy_from_slice(&checksum.to_be_bytes());

    header_len + payload.len()
}

fn get_checksum(packet: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;

    while i + 1 < packet.len() {
        let word = if i == 2 {
            0u16 // Skip checksum field (2..=3)
        } else {
            u16::from_be_bytes([packet[i], packet[i + 1]])
        };
        sum = sum.wrapping_add(word as u32);
        i += 2;
    }

    if i < packet.len() {
        // Odd length: last byte padded with 0
        let word = (packet[i] as u16) << 8;
        sum = sum.wrapping_add(word as u32);
    }

    // Fold overflows
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

pub fn parse_packet(packet: &[u8]) -> Option<(IcmpHeader, &[u8])> {
    if packet.len() < 8 {
        return None; // ICMP header is at least 8 bytes
    }

    let header = unsafe {
        let ptr = packet.as_ptr() as *const IcmpHeader;
        ptr.read_unaligned()
    };

    let header_len = 8; // ICMP header length is always 8 bytes
    let payload = &packet[header_len..];

    Some((header, payload))
}

