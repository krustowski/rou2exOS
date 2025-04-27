use crate::net::ipv4;

#[repr(C, packed)]
pub struct IcmpHeader {
    icmp_type: u8,
    icmp_code: u8,
    checksum: u16,
    identifier: u16,
    sequence_number: u16,
}

pub fn create_packet(
    packet_type: u8,           // 8 for Echo Request, 0 for Echo Reply
    identifier: u16,
    sequence_number: u16,
    payload: &[u8],
    out_buffer: &mut [u8],
) -> usize {
    let header_len = 8; // ICMP header is always 8 bytes

    let mut header = IcmpHeader {
        icmp_type: packet_type,
        icmp_code: 0, // Usually 0 for Echo Request/Reply
        checksum: 0,  // will calculate checksum later
        identifier,
        sequence_number,
    };

    // Copy header into buffer
    unsafe {
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<IcmpHeader>(),
        );
        out_buffer[..header_bytes.len()].copy_from_slice(header_bytes);
    }

    // Calculate checksum
    let checksum = get_checksum(&out_buffer[..header_len]);
    out_buffer[2..4].copy_from_slice(&checksum.to_be_bytes());

    // Copy payload (data) to buffer
    out_buffer[header_len..header_len + payload.len()].copy_from_slice(payload);

    header_len + payload.len()
}

fn get_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(2);

    // Sum the 16-bit words
    for chunk in &mut chunks {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        sum = sum.wrapping_add(word);
    }

    // Handle the odd byte (if any)
    if let Some(&byte) = chunks.remainder().first() {
        let word = (byte as u16) << 8;
        sum = sum.wrapping_add(word as u32);
    }

    // Fold any overflow into the lower 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)  // Final negation for checksum
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

