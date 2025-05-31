use crate::{net::ethernet, vga::write::string};

pub fn handle_packet(vga_index: &mut isize) {
    fn callback(packet: &[u8]) -> u8 {
        1
    }
    if ethernet::receive_loop(callback) == 1 {
        string(vga_index, b"Reveived an Ethernet frame!", crate::vga::buffer::Color::Green);
    }
}
