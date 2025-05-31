use crate::input::port;
use crate::net::serial;
use crate::net::slip;
use crate::net::tcp;

pub const MAX_CONNS: usize = 10;

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

        if let Some(slice) = out_buffer.get_mut(..header_bytes.len()) {
            slice.copy_from_slice(header_bytes);
        }
    }

    // Calculate checksum

    let out_slice = out_buffer.get(..header_len).unwrap_or(&[]);
    let checksum = ipv4_checksum(out_slice);

    if let Some(slice) = out_buffer.get_mut(10..12) {
        slice.copy_from_slice(&checksum.to_be_bytes());
    }

    // Copy payload

    if let Some(slice) = out_buffer.get_mut(header_len..header_len + payload.len()) {
        slice.copy_from_slice(payload);
    }

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

    let payload_slice = packet.get(header_len as usize..).unwrap_or(&[]);

    Some((header, payload_slice))
}


/// Compute IPv4 header checksum
fn ipv4_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(2);

    for chunk in &mut chunks {
        if let Some(w1) = chunk.first() {
            if let Some(w2) = chunk.get(1) {
                sum = sum.wrapping_add( u16::from_be_bytes([*w1, *w2]) as u32 );
            }
        }
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

        let encoded_slice = encoded_buf.get(..encoded_len).unwrap_or(&[]);
        for &b in encoded_slice {
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
        // While the keyboard is idle...
        while port::read(0x64) & 1 == 0 {
            if serial::ready() && temp_len <= temp_buf.len() {

                if let Some(p) = temp_buf.get_mut(temp_len) {
                    *p = serial::read();
                }
                temp_len += 1;

                let temp_slice = temp_buf.get(..temp_len).unwrap_or(&[]);

                if let Some(packet_len) = slip::decode(temp_slice, &mut packet_buf) {
                    // Full packet decoded
                    let packet_slice = packet_buf.get(..packet_len).unwrap_or(&[]);
                    return callback(packet_slice);
                }
            }
        }

        // If any key is pressed, break the loop and return.
        if port::read(0x60) & 0x80 == 0 {
            break;
        }
    }
    3
}

/// Called when you receive a new serial byte
pub fn receive_loop_tcp(conns: &mut [Option<tcp::TcpConnection>; MAX_CONNS], callback: fn(conns: &mut [Option<tcp::TcpConnection>; MAX_CONNS], packet: &[u8]) -> u8) -> u8 {
    let mut temp_buf: [u8; 2048] = [0; 2048];
    let mut packet_buf: [u8; 2048] = [0; 2048];
    let mut temp_len: usize = 0;

    serial::init();

    loop {
        // While the keyboard is idle...
        while port::read(0x64) & 1 == 0 {
            if serial::ready() &&  temp_len <= temp_buf.len() {
                if let Some(p) = temp_buf.get_mut(temp_len) {
                    *p = serial::read();
                }
                temp_len += 1;

                let temp_slice = temp_buf.get(..temp_len).unwrap_or(&[]);

                if let Some(packet_len) = slip::decode(temp_slice, &mut packet_buf) {
                    // Full packet decoded
                    let packet_slice = packet_buf.get(..packet_len).unwrap_or(&[]);
                    return callback(conns, packet_slice);
                }
            }
        }

        // If any key is pressed, break the loop and return.
        if port::read(0x60) & 0x80 == 0 {
            break;
        }
    }
    3
}

