pub const FIN: u16 = 0x01;
pub const SYN: u16 = 0x02;
//pub const RST: u16 = 0x04;
pub const PSH: u16 = 0x08;
pub const ACK: u16 = 0x10;

#[repr(C, packed)]
pub struct TcpHeader {
    pub source_port: u16,
    pub dest_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset_reserved_flags: u16,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
    // Options skipped
}

#[derive(PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    Closing,
    TimeWait,
    CloseWait,
    LastAck,
}

pub struct TcpConnection {
    pub state: TcpState,
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
}

#[allow(clippy::too_many_arguments)]
pub fn create_packet(
    src_port: u16,
    dst_port: u16,
    seq_num: u32,
    ack_num: u32,
    flags: u16,
    window_size: u16,
    payload: &[u8],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    out: &mut [u8],
) -> usize {
    let data_offset = 5u16 << 12; // 5 * 4 = 20 bytes, no options
                                  //
    let tcp_header = TcpHeader {
        source_port: src_port.to_be(),
        dest_port: dst_port.to_be(),
        seq_num: seq_num.to_be(),
        ack_num: ack_num.to_be(),
        data_offset_reserved_flags: (data_offset | flags).to_be(),
        window_size: window_size.to_be(),
        checksum: 0,
        urgent_pointer: 0,
    };

    let header_bytes = unsafe {
        core::slice::from_raw_parts(
            &tcp_header as *const _ as *const u8,
            core::mem::size_of::<TcpHeader>(),
        )
    };

    if let Some(slice) = out.get_mut(..header_bytes.len()) {
        slice.copy_from_slice(header_bytes);
    }
    if let Some(slice) = out.get_mut(20..20 + payload.len()) {
        slice.copy_from_slice(payload);
    }

    let mut checksum: u16 = 0;

    if let Some(out_slice) = out.get_mut(..20 + payload.len()) {
        checksum = get_checksum(src_ip, dst_ip, out_slice);
    }

    if let Some(w) = out.get_mut(16) {
        *w = (checksum >> 8) as u8;
    }
    if let Some(w) = out.get_mut(17) {
        *w = (checksum & 0xff) as u8;
    }

    20 + payload.len()
}


pub fn parse_packet(packet: &[u8]) -> Option<(TcpHeader, &[u8])> {
    if packet.len() < 20 {
        return None;
    }

    let header = unsafe {
        core::ptr::read_unaligned(packet.as_ptr() as *const TcpHeader)
    };

    let data_offset = (u16::from_be(header.data_offset_reserved_flags) >> 12) * 4;
    if packet.len() < data_offset as usize {
        return None;
    }

    //let payload = &packet[data_offset as usize..];
    let payload_slice = packet.get(data_offset as usize..).unwrap_or(&[]);
    Some((header, payload_slice))
}

pub fn parse_flags(header: &TcpHeader) -> (bool, bool, bool, bool) {
    //let flags = header.data_offset_reserved_flags & 0x01FF;
    let flags = u16::from_be(header.data_offset_reserved_flags) & 0x01FF;

    let fin = flags & 0x001 != 0;
    let syn = flags & 0x002 != 0;
    let rst = flags & 0x004 != 0;
    let ack = flags & 0x010 != 0;
    (syn, ack, fin, rst)
}

pub fn get_checksum(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    tcp_packet: &[u8],
) -> u16 {
    let mut sum: u32 = 0;

    // Pseudo-header: Source IP (4), Dest IP (4), zero (1), protocol (1), TCP length (2)

    if let Some(w1) = src_ip.first() {
        if let Some(w2) = src_ip.get(1) {
            sum += u16::from_be_bytes([*w1, *w2]) as u32;
        }
    }
    if let Some(w1) = src_ip.get(2) {
        if let Some(w2) = src_ip.get(3) {
            sum += u16::from_be_bytes([*w1, *w2]) as u32;
        }
    }

    if let Some(w1) = dst_ip.first() {
        if let Some(w2) = dst_ip.get(1) {
            sum += u16::from_be_bytes([*w1, *w2]) as u32;
        }
    }
    if let Some(w1) = dst_ip.get(2) {
        if let Some(w2) = dst_ip.get(3) {
            sum += u16::from_be_bytes([*w1, *w2]) as u32;
        }
    }

    sum += 0x0006u16 as u32; // Protocol: TCP = 6
    sum += tcp_packet.len() as u32;

    // Now include the TCP header + payload
    let mut i = 0;
    while i + 1 < tcp_packet.len() {
        // Skip checksum field at offset 16..18
        if i == 16 {
            i += 2;
            continue;
        }

        if let Some(w1) = tcp_packet.get(i) {
            if let Some(w2) = tcp_packet.get(i+1) {
                sum = sum.wrapping_add(u16::from_be_bytes([*w1, *w2]) as u32);
            }
        }
        i += 2;
    }

    if i < tcp_packet.len() {
        // Odd byte at the end
        if let Some(w) = tcp_packet.get(i) {
            sum = sum.wrapping_add(((*w as u16) << 8) as u32);
        }
        //let word = (tcp_packet[i] as u16) << 8;
        //sum = sum.wrapping_add(word as u32);
    }

    // Fold 32-bit sum to 16 bits
    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

