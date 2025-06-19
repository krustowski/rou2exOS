use crate::net::ipv4;
use crate::net::udp;

pub fn udp_handler(ipv4_header: &ipv4::Ipv4Header, ipv4_payload: &[u8]) -> u8 {
    if ipv4_header.protocol != 17 { 
        return 2;
    }

    if let Some((src_port, dst_port, payload)) = udp::parse_packet(ipv4_payload) {
        if dst_port == 80 && payload.starts_with(b"GET /") {
            let http_response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from rou2exOS Rusted Edition!";

            let mut udp_buf = [0u8; 512];
            let udp_len = udp::create_packet(
                [192, 168, 3, 2],    // Local IP
                ipv4_header.source_ip,
                dst_port,           // From our 80/8080
                src_port,             // Back to client
                http_response,
                &mut udp_buf,
            );

            let udp_slice = udp_buf.get(..udp_len).unwrap_or(&[]);

            // Now calculate checksum
            let checksum = udp::get_checksum(
                [192, 168, 3, 2], // Source IP
                [192, 168, 3, 1], // Destination IP
                udp_slice,
            );

            // Insert checksum into udp_buf
            if let Some(slice) = udp_buf.get_mut(6..8) {
                slice.copy_from_slice(&checksum.to_be_bytes());
            }

            let mut ipv4_buf = [0u8; 1500];

            let udp_slice = udp_buf.get(..udp_len).unwrap_or(&[]);
            let ipv4_len = ipv4::create_packet(
                [192, 168, 3, 2],
                ipv4_header.source_ip,
                17, // UDP
                udp_slice,
                &mut ipv4_buf,
            );

            // Send the IPv4 packet
            let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);
            ipv4::send_packet(ipv4_slice);

            return 0;
        }
    }
    2
}
