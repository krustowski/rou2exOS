use crate::net::ipv4;
use crate::net::tcp;
use crate::vga;

const MAX_CONNS: usize = 4;

pub fn handle(vga_index: &mut isize) {
    fn callback(conns: &mut [Option<tcp::TcpConnection>; MAX_CONNS], packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = ipv4::parse_packet(packet) {
            if let Some((tcp_header, payload)) = tcp::parse_packet(ipv4_payload) {
                let (syn, ack, _fin, _rst) = tcp::parse_flags(&tcp_header);

                let src_ip = ipv4_header.source_ip;
                let dst_ip = ipv4_header.dest_ip;
                let src_port = u16::from_be(tcp_header.source_port);
                let dst_port = u16::from_be(tcp_header.dest_port);

                for conn in conns.iter_mut() {
                    if let Some(c) = conn {
                        if c.state == tcp::TcpState::Closed || c.state == tcp::TcpState::CloseWait || c.state == tcp::TcpState::FinWait1 || c.state == tcp::TcpState::FinWait2 || c.state == tcp::TcpState::Closing || c.state == tcp::TcpState::TimeWait || c.state == tcp::TcpState::LastAck {
                            *conn = None;
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
                    slot.as_mut().unwrap()
                } else if syn && !ack {
                    // New connection
                    if let Some(empty_slot) = conns.iter_mut().find(|c| c.is_none()) {
                        *empty_slot = Some(tcp::TcpConnection {
                            state: tcp::TcpState::Listen,
                            src_ip: src_ip,
                            dst_ip: dst_ip,
                            src_port: dst_port,
                            dst_port: src_port,
                            seq_num: 0,
                            ack_num: 0,
                            //peer_seq_num: 0,
                        });
                        empty_slot.as_mut().unwrap()
                    } else {
                        // No free slot — drop the packet
                        return 255;
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

    let mut conns: [Option<tcp::TcpConnection>; MAX_CONNS] = [None, None, None, None];

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Starting a simple TCP tester (hit any key to interrupt)...", 0x0f);
    vga::write::newline(vga_index);

    loop {
        let ret = ipv4::receive_loop_tcp(&mut conns, callback);

        if ret == 0 {
            vga::write::string(vga_index, b"Received SYN", 0x0f);
            vga::write::newline(vga_index);

        } else if ret == 1 {
            vga::write::string(vga_index, b"Received ACK", 0x0f);
            vga::write::newline(vga_index);
        } else if ret == 2 {
            vga::write::string(vga_index, b"Received FIN", 0x0f);
            vga::write::newline(vga_index);
            //break;
        } else if ret == 3 {
            vga::write::string(vga_index, b"Keyboard interrupt", 0x0f);
            vga::write::newline(vga_index);
            break;
        } else if ret == 253 {
            vga::write::string(vga_index, b"Freed socket", 0x0f);
            vga::write::newline(vga_index);
        }
    }
}

fn handle_tcp_packet(conn: &mut tcp::TcpConnection, tcp_header: &tcp::TcpHeader, payload: &[u8]) -> u8 {
    let (syn, ack, fin, _rst) = tcp::parse_flags(&tcp_header);

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
        if payload.len() > 0 {
            conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(payload.len() as u32);

            // Try to parse a minimal GET request
            if payload.starts_with(b"GET /") {
                let mut http_response = [0u8; 1024];
                let http_len = http_router(&payload, &mut http_response);

                send_response(conn, tcp::ACK | tcp::PSH | tcp::FIN, &http_response[..http_len]);
                conn.seq_num += http_len as u32;

                conn.state = tcp::TcpState::CloseWait;
                return 2;
            }

            // Fallback: Echo
            let mut reply = [0u8; 128];
            let msg = b"Echo: ";
            reply[..msg.len()].copy_from_slice(msg);
            let len = core::cmp::min(payload.len(), reply.len() - msg.len());
            reply[msg.len()..msg.len() + len].copy_from_slice(&payload[..len]);

            send_response(conn, tcp::ACK | tcp::PSH, &reply[..msg.len() + len]);

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

    return 1
}

fn send_response(conn: &mut tcp::TcpConnection, flags: u16, payload: &[u8]) {
    let mut out_buf = [0u8; 500];
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

    let ipv4_len = ipv4::create_packet(conn.dst_ip, conn.src_ip, 6, &out_buf[..tcp_len], &mut ipv4_buf);
    ipv4::send_packet(&ipv4_buf[..ipv4_len]);
}

fn u32_to_ascii(mut num: u32, buf: &mut [u8]) -> usize {
    let mut digits = [0u8; 10];
    let mut i = 0;
    if num == 0 {
        buf[0] = b'0';
        return 1;
    }
    while num > 0 {
        digits[i] = b'0' + (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    // Reverse digits into buf
    for j in 0..i {
        buf[j] = digits[i - j - 1];
    }
    i
}

fn http_router(payload: &[u8], http_response: &mut [u8]) -> usize {
        let body: &str;
        let mut content_type: &str = "";

        if payload.starts_with(b"GET / ") || payload.starts_with(b"GET / HTTP/1.1") {
            body = "<html><body><h1>Welcome to RoureXOS</h1></body></html>";
            content_type = "text/html";

        } else if payload.starts_with(b"GET /hello") {
            body = "Hello World from RoureXOS!";
            content_type = "text/plain";

        } else if payload.starts_with(b"GET /json") {
            body = "{\"message\":\"Hello JSON\"}";
            content_type = "application/json";

        } else {
            body = "404 Not Found";
        }

        let body_len = body.len();
        let header = b"HTTP/1.1 200 OK\r\nContent-Type: ";
        let mut pos = 0;

        http_response[..header.len()].copy_from_slice(header);
        pos += header.len();

        http_response[pos..pos + content_type.len()].copy_from_slice(content_type.as_bytes());
        pos += content_type.len();

        http_response[pos..pos + 2].copy_from_slice(b"\r\n");
        pos += 2;

        http_response[pos..pos + 16].copy_from_slice(b"Content-Length: ");
        pos += 16;

        pos += u32_to_ascii(body_len as u32, &mut http_response[pos..]);

        http_response[pos..pos + 4].copy_from_slice(b"\r\n\r\n");
        pos += 4;

        http_response[pos..pos + body_len].copy_from_slice(body.as_bytes());
        pos += body_len;

        pos
}
