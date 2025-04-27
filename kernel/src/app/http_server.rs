use crate::net;

pub fn udp_handler(ipv4_header: &net::ipv4::Ipv4Header, ipv4_payload: &[u8]) -> u8 {
    if ipv4_header.protocol != 17 { 
        return 2;
    }

    if let Some((src_port, dst_port, payload)) = net::udp::parse_packet(ipv4_payload) {
        if dst_port == 80 {
            if payload.starts_with(b"GET /") {
                let http_response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from rou2exOS Rusted Edition!";

                let mut udp_buf = [0u8; 512];
                let udp_len = net::udp::create_packet(
                    [192, 168, 3, 2],    // our IP
                    ipv4_header.source_ip,
                    dst_port,            // from our 80/8080
                    src_port,            // back to client
                    http_response,
                    &mut udp_buf,
                );

                let mut ipv4_buf = [0u8; 1500];
                let ipv4_len = net::ipv4::create_packet(
                    [192, 168, 3, 2],
                    ipv4_header.source_ip,
                    17, // UDP
                    &udp_buf[..udp_len],
                    &mut ipv4_buf,
                );

                net::ipv4::send_packet(&ipv4_buf[..ipv4_len]);

                return 0;
            }
        }
    }
    2
}
