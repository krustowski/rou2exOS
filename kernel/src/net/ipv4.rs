use crate::net::serial;
use crate::net::slip;

#[repr(C, packed)]
pub struct Ipv4Header {
    version_ihl: u8,
    dscp_ecn: u8,
    total_length: u16,
    identification: u16,
    flags_fragment_offset: u16,
    ttl: u8,
    pub protocol: u8,
    header_checksum: u16,
    pub source_ip: [u8; 4],
    pub dest_ip: [u8; 4],
}

//
//  CREATE/HANDLE PACKET
//

pub fn create_packet(
    source: [u8; 4],
    dest: [u8; 4],
    protocol: u8,
    payload: &[u8],
    out_buffer: &mut [u8],
) -> usize {
    let header_len = 20;
    let total_len = (header_len + payload.len()) as u16;

    let header = Ipv4Header {
        version_ihl: (4 << 4) | 5, // Version 4, IHL=5 (20 bytes)
        dscp_ecn: 0,
        total_length: total_len.to_be(),
        identification: 0x1337u16.to_be(),
        flags_fragment_offset: (0x4000u16).to_be(), // Don't Fragment flag
        ttl: 64,
        protocol,
        header_checksum: 0, // will fix later
        source_ip: source,
        dest_ip: dest,
    };

    // Copy header into buffer
    unsafe {
        let header_bytes = core::slice::from_raw_parts(
            &header as *const _ as *const u8,
            core::mem::size_of::<Ipv4Header>(),
        );
        out_buffer[..header_bytes.len()].copy_from_slice(header_bytes);
    }

    // Calculate checksum
    let checksum = ipv4_checksum(&out_buffer[..header_len]);
    out_buffer[10..12].copy_from_slice(&checksum.to_be_bytes());

    // Copy payload
    out_buffer[header_len..header_len + payload.len()].copy_from_slice(payload);

    header_len + payload.len()
}

pub fn parse_packet(packet: &[u8]) -> Option<(Ipv4Header, &[u8])> {
    if packet.len() < 20 {
        return None;
    }

    let header = unsafe {
        let ptr = packet.as_ptr() as *const Ipv4Header;
        ptr.read_unaligned()
    };

    let header_len = (header.version_ihl & 0x0F) * 4;
    if packet.len() < header_len as usize {
        return None;
    }

    let payload = &packet[header_len as usize..];

    Some((header, payload))
}


/// Compute IPv4 header checksum
fn ipv4_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(2);

    for chunk in &mut chunks {
        let word = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
        sum = sum.wrapping_add(word);
    }
    if let Some(&byte) = chunks.remainder().first() {
        let word = (byte as u16) << 8;
        sum = sum.wrapping_add(word as u32);
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

//
//  SEND/RECEIVE PACKET
//

/// Called to send a packet
pub fn send_packet(packet: &[u8]) {
    let mut encoded_buf = [0u8; 4096];

    if let Some(encoded_len) = slip::encode(packet, &mut encoded_buf) {
        serial::init();

        for &b in &encoded_buf[..encoded_len] {
            serial::write(b);
        }
    }
}

/// Called when you receive a new serial byte
pub fn receive_loop(callback: fn(packet: &[u8]) -> u8) -> u8 {
    let mut temp_buf: [u8; 2048] = [0; 2048];
    let mut packet_buf: [u8; 2048] = [0; 2048];
    let mut temp_len: usize = 0;

    serial::init();

    loop {
        if serial::ready() {
            if temp_len <= temp_buf.len() {
                temp_buf[temp_len] = serial::read();
                temp_len += 1;

                if let Some(packet_len) = slip::decode(&mut temp_buf[..temp_len], &mut packet_buf) {
                    // Full packet decoded
                    return callback(&packet_buf[..packet_len]);
                }
            }
        }
    }
}

