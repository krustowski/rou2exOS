use crate::{net::ethernet};

pub fn handle_packet() {
    fn callback(_packet: &[u8]) -> u8 {
        1
    }
    if ethernet::receive_loop(callback) == 1 {
        println!("Reveived an Ethernet frame!")
    }
}
