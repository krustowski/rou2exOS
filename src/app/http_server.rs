use crate::net::ipv4;
//use crate::net::tcp;
use crate::net::udp;

//const LISTEN_PORT: u16 = 8080;

/*fn handle_http_request(request: &[u8]) -> &[u8] {
  if request.starts_with(b"GET / HTTP/1.1") {
  return b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from tiny Rust kernel!";
  }
  b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\n\r\nPage Not Found"
  }*/

/*pub fn listen_for_http() {
// Main loop
loop {
let packet = ipv4::receive_packet();

if let Some(tcp_header) = tcp::parse_header(&packet) {
if tcp_header.dst_port == LISTEN_PORT {
// Step 1: TCP SYN (client is trying to initiate a connection)
if tcp_header.syn {
// Send SYN/ACK response
tcp::send_syn_ack(&tcp_header);
}

// Step 2: TCP ACK (client has acknowledged our SYN/ACK)
if tcp_header.ack {
// Read the incoming HTTP request from the packet payload
let data = tcp::read_data(&packet);
let response = handle_http_request(data);

// Step 3: Send the HTTP response back to the client
tcp::send_data(&tcp_header, response);
}
}
}
}
}*/

pub fn udp_handler(ipv4_header: &ipv4::Ipv4Header, ipv4_payload: &[u8]) -> u8 {
    if ipv4_header.protocol != 17 { 
        return 2;
    }

    if let Some((src_port, dst_port, payload)) = udp::parse_packet(ipv4_payload) {
        if dst_port == 80 && payload.starts_with(b"GET /") {
            let http_response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from rou2exOS Rusted Edition!";

            let mut udp_buf = [0u8; 512];
            let udp_len = udp::create_packet(
                [192, 168, 3, 2],    // our IP
                ipv4_header.source_ip,
                dst_port,            // from our 80/8080
                src_port,            // back to client
                http_response,
                &mut udp_buf,
            );

            let udp_slice = udp_buf.get(..udp_len).unwrap_or(&[]);

            // Now calculate checksum
            let checksum = udp::get_checksum(
                [192, 168, 3, 2], // source IP
                [192, 168, 3, 1], // destination IP
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

            let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);
            ipv4::send_packet(ipv4_slice);

            return 0;
        }
    }
    2
}
