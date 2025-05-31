use crate::net::ipv4;
use crate::net::tcp;
use crate::vga;

pub fn handle(vga_index: &mut isize) {
    fn callback(conns: &mut [Option<tcp::TcpConnection>; ipv4::MAX_CONNS], packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = ipv4::parse_packet(packet) {
            if let Some((tcp_header, payload)) = tcp::parse_packet(ipv4_payload) {
                let (syn, ack, _fin, _rst) = tcp::parse_flags(&tcp_header);

                let src_ip = ipv4_header.source_ip;
                let dst_ip = ipv4_header.dest_ip;
                let src_port = u16::from_be(tcp_header.source_port);
                let dst_port = u16::from_be(tcp_header.dest_port);

                for conn in conns.iter_mut() {
                    if let Some(c) = conn {
                        if c.state == tcp::TcpState::Closed || 
                            c.state == tcp::TcpState::CloseWait || 
                                c.state == tcp::TcpState::FinWait1 || 
                                c.state == tcp::TcpState::FinWait2 || 
                                c.state == tcp::TcpState::Closing || 
                                c.state == tcp::TcpState::TimeWait || 
                                c.state == tcp::TcpState::LastAck {
                                    *conn = None;
                                    return 253;
                        }
                    }
                }

                // Find a conn
                let maybe_existing = conns.iter_mut().find(|entry| {
                    if let Some(conn) = entry {
                        conn.src_ip == src_ip &&
                            conn.dst_ip == dst_ip &&
                            conn.src_port == dst_port &&
                            conn.dst_port == src_port
                    } else {
                        false
                    }
                });

                let conn = if let Some(slot) = maybe_existing {
                    match slot.as_mut() {
                        Some(c) => c,
                        None => return 253, // Unexpected: maybe_existing was Some, but inner value was None
                    }
                } else if syn && !ack {
                    // New connection
                    match conns.iter_mut().find(|c| c.is_none()) {
                        Some(empty_slot) => {
                            *empty_slot = Some(tcp::TcpConnection {
                                state: tcp::TcpState::Listen,
                                src_ip,
                                dst_ip,
                                src_port: dst_port,
                                dst_port: src_port,
                                seq_num: 0,
                                ack_num: 0,
                            });

                            match empty_slot.as_mut() {
                                Some(c) => c,
                                None => return 252, // This shouldn't happen, just inserted Some
                            }
                        }
                        None => return 255, // No free slot — drop the packet
                    }
                } else {
                    // Packet for unknown connection — drop
                    return 254;
                };


                return handle_tcp_packet(conn, &tcp_header, payload);
            }
        }
        2
    }

    //let mut conns: [Option<tcp::TcpConnection>; ipv4::MAX_CONNS] = [None, None, None, None];
    let mut conns: [Option<tcp::TcpConnection>; ipv4::MAX_CONNS] = Default::default();

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Starting a simple TCP tester (hit any key to interrupt)...", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    loop {
        let ret = ipv4::receive_loop_tcp(&mut conns, callback);

        if ret == 0 {
            vga::write::string(vga_index, b"Received SYN", vga::buffer::Color::White);
            vga::write::newline(vga_index);

        } else if ret == 1 {
            vga::write::string(vga_index, b"Received ACK", vga::buffer::Color::White);
            vga::write::newline(vga_index);
        } else if ret == 2 {
            vga::write::string(vga_index, b"Received FIN", vga::buffer::Color::White);
            vga::write::newline(vga_index);
            //break;
        } else if ret == 3 {
            vga::write::string(vga_index, b"Keyboard interrupt", vga::buffer::Color::White);
            vga::write::newline(vga_index);
            break;
        } else if ret == 253 {
            vga::write::string(vga_index, b"Freed socket", vga::buffer::Color::White);
            vga::write::newline(vga_index);
        } else if ret == 254 {
            vga::write::string(vga_index, b"Unknown conn", vga::buffer::Color::White);
            vga::write::newline(vga_index);
        } else if ret == 255 {
            vga::write::string(vga_index, b"No free slots", vga::buffer::Color::White);
            vga::write::newline(vga_index);
        }
    }
}

fn handle_tcp_packet(conn: &mut tcp::TcpConnection, tcp_header: &tcp::TcpHeader, payload: &[u8]) -> u8 {
    let (syn, ack, fin, _rst) = tcp::parse_flags(tcp_header);

    if syn && !ack {
        conn.src_port = u16::from_be(tcp_header.dest_port);
        conn.dst_port = u16::from_be(tcp_header.source_port);

        // SYN received, reply with SYN+ACK
        conn.state = tcp::TcpState::SynReceived;
        conn.seq_num = 1;
        conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(1);

        send_response(conn, tcp::SYN | tcp::ACK, payload);
        return 0;
    } 

    if ack && conn.state == tcp::TcpState::SynReceived {
        // Ready to receive/send data
        conn.state = tcp::TcpState::Established;
        conn.ack_num = u32::from_be(tcp_header.seq_num);
        conn.seq_num += 1;

        //send_response(conn, tcp::ACK, b"Connection established\r\n");
        send_response(conn, tcp::ACK, &[]);
        return 1;

    } else if conn.state == tcp::TcpState::Established {
        if payload.is_empty() {
            conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(payload.len() as u32);

            // Try to parse a minimal GET request
            if payload.starts_with(b"GET /") {
                let mut http_response = [0u8; 1420];
                let http_len = http_router(payload, &mut http_response);

                let http_slice = http_response.get(..http_len).unwrap_or(&[]);
                send_response(conn, tcp::ACK | tcp::PSH | tcp::FIN, http_slice);
                conn.seq_num += http_len as u32;

                conn.state = tcp::TcpState::CloseWait;
                return 2;
            }

            // Fallback: Echo
            let mut reply = [0u8; 128];
            let msg = b"Echo: ";

            if let Some(slice) = reply.get_mut(..msg.len()) {
                slice.copy_from_slice(msg);
            }

            let len = core::cmp::min(payload.len(), reply.len() - msg.len());

            if let Some(slice) = reply.get_mut(msg.len()..msg.len() + len) {
                let payload_slice = payload.get(..len).unwrap_or(&[]);
                slice.copy_from_slice(payload_slice);
            }

            let reply_slice = reply.get(..msg.len() + len).unwrap_or(&[]);
            send_response(conn, tcp::ACK | tcp::PSH, reply_slice);

            conn.seq_num += (msg.len() + len) as u32;
        } else {
            if fin {
                conn.state = tcp::TcpState::Closed;
                conn.ack_num = u32::from_be(tcp_header.seq_num);

                send_response(conn, tcp::FIN | tcp::ACK, &[]);
                return 2;
            }

            // Just ACK to keep connection alive
            conn.ack_num = u32::from_be(tcp_header.seq_num);
            send_response(conn, tcp::ACK, &[]);
        }
    }

    1
}

fn send_response(conn: &mut tcp::TcpConnection, flags: u16, payload: &[u8]) {
    let mut out_buf = [0u8; 1420];
    let mut ipv4_buf = [0u8; 1500];

    let tcp_len = tcp::create_packet(
        conn.src_port,
        conn.dst_port,
        conn.seq_num,
        conn.ack_num,
        flags,
        1024,
        payload,
        conn.src_ip,
        conn.dst_ip,
        &mut out_buf
    );

    let tcp_slice = out_buf.get(..tcp_len).unwrap_or(&[]);
    let ipv4_len = ipv4::create_packet(conn.dst_ip, conn.src_ip, 6, tcp_slice, &mut ipv4_buf);

    let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);
    ipv4::send_packet(ipv4_slice);
}

fn u32_to_ascii(mut num: u32, buf: &mut [u8]) -> usize {
    let mut digits = [0u8; 10];
    let mut i = 0;
    if num == 0 {
        if let Some(c) = buf.get_mut(0) {
            *c = b'0';
        }
        return 1;
    }
    while num > 0 {
        if let Some(d) = digits.get_mut(i) {
            *d = b'0' + (num % 10) as u8;
        }
        num /= 10;
        i += 1;
    }
    // Reverse digits into buf
    for j in 0..i {
        if let Some(c) = buf.get_mut(j) {
            if let Some(d) = digits.get(i - j - 1) {
                *c = *d;
            }
        }
    }
    i
}

/*fn write_u32(buf: &mut [u8], mut idx: usize, mut num: u32) -> usize {
//let start = idx;
let mut rev = [0u8; 10]; // max digits in u32
let mut i = 0;

if num == 0 {
buf[idx] = b'0';
return idx + 1;
}

while num > 0 {
rev[i] = b'0' + (num % 10) as u8;
num /= 10;
i += 1;
}

while i > 0 {
i -= 1;
buf[idx] = rev[i];
idx += 1;
}

idx
}*/

fn match_path(payload: &[u8], path: &[u8]) -> bool {
    if payload.starts_with(b"GET ") {
        let slice = payload.get(4..).unwrap_or(&[]);

        let mut i = 0;

        if let Some(w) = slice.get(i) {
            while i < slice.len() && *w != b' ' {
                i += 1;
            }
        }

        let sl = slice.get(..i).unwrap_or(&[]);
        sl == path
    } else {
        false
    }
}

fn http_router(payload: &[u8], http_response: &mut [u8]) -> usize {
    let body: &str;
    let mut content_type: &str = "text/plain";

    if match_path(payload, b"/") || payload.starts_with(b"GET / HTTP/1.1") {
        body = "<html><body><h1>Welcome to rou2exOS HTTP server</h1></body></html>";
        content_type = "text/html";

    } else if match_path(payload, b"/rouring") {
        body = "<html><head><style>.index {width: 800px;margin-top: 70px;font-family: Helvetica;}.lefts2 {width: 200px;float: left;}.rights2 {width: 590px;float: right;}.foot {width: 550px;margin-top: 200px;font-family: Helvetica;clear: both;}</style><meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\"><meta http-equiv=\"Content-language\" content=\"cs, en\"><title>The RouRa Project</title></head><body><center><div class=\"index\"><div class=\"lefts2\"><br><img src=\"https://rouring.net/plug.png\" width=\"200\"></div><div class=\"rights2\"><br><p style=\"font-size: 42px\">The RouRa Project</p><p style=\"font-size: 20px\">Už bude zase dobře</p></div></div><div class=\"foot\"><br><br><br><br>Rouring.net & ReRour 2k16</div></body></center></html>";
        content_type = "text/html";

    } else if match_path(payload, b"/json") {
        body = "{\"message\":\"Hello JSON\"}";
        content_type = "application/json";

    } else {
        body = "404 Not Found";
    }

    let body_len = body.len();
    let header = b"HTTP/1.1 200 OK\r\nContent-Type: ";
    let mut pos = 0;

    if let Some(slice) = http_response.get_mut(..header.len()) {
        slice.copy_from_slice(header);
    }
    pos += header.len();

    if let Some(slice) = http_response.get_mut(pos..pos + content_type.len()) {
        slice.copy_from_slice(content_type.as_bytes());
    }
    pos += content_type.len();

    if let Some(slice) = http_response.get_mut(pos..pos + 2) {
        slice.copy_from_slice(b"\r\n");
    }
    pos += 2;

    if let Some(slice) = http_response.get_mut(pos..pos + 16) {
        slice.copy_from_slice(b"Content-Length: ");
    }
    pos += 16;

    let response_slice = http_response.get_mut(pos..).unwrap_or(&mut []);
    pos += u32_to_ascii(body_len as u32, response_slice);

    if let Some(slice) = http_response.get_mut(pos..pos + 4) {
        slice.copy_from_slice(b"\r\n\r\n");
    }
    pos += 4;

    if let Some(slice) = http_response.get_mut(pos..pos + body_len) {
        slice.copy_from_slice(body.as_bytes());
    }
    pos += body_len;
    //http_response[pos..pos + body_pos].copy_from_slice(&body_slice);
    //pos += body_pos;

    pos
}
