pub struct EthernetFrame<'a> {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
    pub payload: &'a [u8],
}

impl<'a> EthernetFrame<'a> {
    pub fn from_bytes(frame: &'a [u8]) -> Option<Self> {
        if frame.len() < 14 {
            return None;
        }
        let dst_mac = frame[0..6].try_into().unwrap();
        let src_mac = frame[6..12].try_into().unwrap();
        let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
        let payload = &frame[14..];
        Some(Self { dst_mac, src_mac, ethertype, payload })
    }
}

