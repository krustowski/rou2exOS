use crate::net::arp::{ArpPacket};
use crate::input::port;

use super::rtl8139;

#[derive(Debug, Clone, Copy)]
pub struct MacAddress(pub [u8; 6]);

#[derive(Debug)]
pub enum EtherType {
    Ipv4,
    Arp,
    Unknown(u16),
}

impl EtherType {
    pub fn from_u16(value: u16) -> Self {
        match value {
            0x0800 => EtherType::Ipv4,
            0x0806 => EtherType::Arp,
            other => EtherType::Unknown(other),
        }
    }

    pub fn to_u16(&self) -> u16 {
        match *self {
            EtherType::Ipv4 => 0x0800,
            EtherType::Arp => 0x0806,
            EtherType::Unknown(val) => val,
        }
    }
}

pub struct EthernetFrame<'a> {
    pub dest_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ethertype: EtherType,
    pub payload: &'a [u8],
}

impl<'a> EthernetFrame<'a> {
    pub fn parse(frame: &'a [u8]) -> Option<Self> {
        if frame.len() < 14 {
            return None;
        }

        let dest_mac = MacAddress([frame[0], frame[1], frame[2], frame[3], frame[4], frame[5]]);
        let src_mac = MacAddress([frame[6], frame[7], frame[8], frame[9], frame[10], frame[11]]);
        let ethertype = EtherType::from_u16(u16::from_be_bytes([frame[12], frame[13]]));
        let payload = &frame[14..];

        Some(Self {
            dest_mac,
            src_mac,
            ethertype,
            payload,
        })
    }

    pub fn write(
        buffer: &mut [u8],
        dest_mac: MacAddress,
        src_mac: MacAddress,
        ethertype: EtherType,
        payload: &[u8],
    ) -> Option<usize> {
        if buffer.len() < 14 + payload.len() {
            return None;
        }

        buffer[0..6].copy_from_slice(&dest_mac.0);
        buffer[6..12].copy_from_slice(&src_mac.0);
        let ethertype_bytes = ethertype.to_u16().to_be_bytes();
        buffer[12..14].copy_from_slice(&ethertype_bytes);
        buffer[14..14 + payload.len()].copy_from_slice(payload);

        Some(14 + payload.len())
    }
}

pub fn receive_frame(buf: &mut [u8]) -> Option<usize> {
    // Hardware-specific receive logic (e.g., RTL8139, E1000)
    // Fill `buf[..len]` with received frame
    // Return Some(len) if a frame is received

    None // placeholder
}

pub fn build_ethernet_frame(
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    ethertype: u16,
    payload: &[u8],
) -> [u8; 1514] {
    let mut buf = [0u8; 1514];
    buf[..6].copy_from_slice(&dst_mac);
    buf[6..12].copy_from_slice(&src_mac);
    buf[12..14].copy_from_slice(&ethertype.to_be_bytes());
    buf[14..14 + payload.len()].copy_from_slice(payload);
    buf
}

fn send_arp_reply(payload: &[u8]) {
    let frame = build_ethernet_frame(
        [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01],
        [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC],
        0x0806,
        &payload,
    );

    rtl8139::send_frame(&frame).unwrap();
}


pub fn receive_loop(callback: fn(packet: &[u8]) -> u8) -> u8 {
    let mut frame_buf: [u8; 2048] = [0; 2048];
            
    loop {
        // While the keyboard is idle...
        while port::read(0x64) & 1 == 0 {

            rtl8139::rtl8139_init();

            if let Some(frame_len) = rtl8139::receive_frame(&mut frame_buf) {
                // Minimal length check
                if frame_len < 14 {
                    continue;
                }

                if let Some(slice) = frame_buf.get(..frame_len) {
                    return callback(slice);
                    if let Some(frame) = EthernetFrame::parse(slice) {

                        match frame.ethertype {
                            //0x0800 => {
                            EtherType::Ipv4 => {
                                // IPv4 packet → pass to callback
                                return callback(frame.payload);
                            }
                            //0x0806 => {
                            EtherType::Arp => {
                                // ARP → handle separately
                                if let Some(arp) = ArpPacket::parse(frame.payload) {
                                    // handle_arp(arp);
                                }
                            }
                            _ => {
                                return callback(frame.payload);
                            }
                        }
                        }
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

