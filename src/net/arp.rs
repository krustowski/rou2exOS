#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Address(pub [u8; 4]);

#[repr(u16)]
pub enum ArpOp {
    Request = 1,
    Reply = 2,
}

pub struct ArpPacket<'a> {
    pub hw_type: u16,       // usually 1 for Ethernet
    pub proto_type: u16,    // usually 0x0800 for IPv4
    pub hw_len: u8,         // 6
    pub proto_len: u8,      // 4
    pub op: ArpOp,
    pub sender_mac: MacAddress,
    pub sender_ip: Ipv4Address,
    pub target_mac: MacAddress,
    pub target_ip: Ipv4Address,
    pub raw: &'a [u8],      // whole packet slice (optional use)
}

impl<'a> ArpPacket<'a> {
    pub fn parse(packet: &'a [u8]) -> Option<Self> {
        if packet.len() < 28 {
            return None;
        }

        let hw_type = u16::from_be_bytes([packet[0], packet[1]]);
        let proto_type = u16::from_be_bytes([packet[2], packet[3]]);
        let hw_len = packet[4];
        let proto_len = packet[5];
        let op_code = u16::from_be_bytes([packet[6], packet[7]]);
        let op = match op_code {
            1 => ArpOp::Request,
            2 => ArpOp::Reply,
            _ => return None,
        };

        let sender_mac = MacAddress([
            packet[8], packet[9], packet[10],
            packet[11], packet[12], packet[13],
        ]);
        let sender_ip = Ipv4Address([packet[14], packet[15], packet[16], packet[17]]);
        let target_mac = MacAddress([
            packet[18], packet[19], packet[20],
            packet[21], packet[22], packet[23],
        ]);
        let target_ip = Ipv4Address([packet[24], packet[25], packet[26], packet[27]]);

        Some(Self {
            hw_type,
            proto_type,
            hw_len,
            proto_len,
            op,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
            raw: packet,
        })
    }

    pub fn build(
        buf: &mut [u8],
        op: ArpOp,
        sender_mac: MacAddress,
        sender_ip: Ipv4Address,
        target_mac: MacAddress,
        target_ip: Ipv4Address,
    ) -> Option<usize> {
        if buf.len() < 28 {
            return None;
        }

        buf[0..2].copy_from_slice(&1u16.to_be_bytes());       // hw type: Ethernet
        buf[2..4].copy_from_slice(&0x0800u16.to_be_bytes());  // proto type: IPv4
        buf[4] = 6;                                            // MAC length
        buf[5] = 4;                                            // IP length
        buf[6..8].copy_from_slice(&(op as u16).to_be_bytes());

        buf[8..14].copy_from_slice(&sender_mac.0);
        buf[14..18].copy_from_slice(&sender_ip.0);
        buf[18..24].copy_from_slice(&target_mac.0);
        buf[24..28].copy_from_slice(&target_ip.0);

        Some(28)
    }
}

