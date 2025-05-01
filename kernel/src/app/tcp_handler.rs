use crate::net::ipv4;
use crate::net::tcp;
use crate::vga;

pub fn handle(vga_index: &mut isize) {
    fn callback(conns: &mut [Option<tcp::TcpConnection>; 4], packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = ipv4::parse_packet(packet) {
            if let Some((tcp_header, payload)) = tcp::parse_packet(ipv4_payload) {
                let (syn, ack, fin, rst) = tcp::parse_flags(&tcp_header);

                let src_ip = ipv4_header.source_ip;
                let dst_ip = ipv4_header.dest_ip;
                let src_port = u16::from_be(tcp_header.source_port);
                let dst_port = u16::from_be(tcp_header.dest_port);

                let mut out_buf = [0u8; 500];
                let mut ipv4_buf = [0u8; 1500];

                let mut tcp_len = 0;

                for conn in conns.iter_mut() {
                    if let Some(c) = conn {
                        if c.state == tcp::TcpState::Closed {
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
                            peer_seq_num: 0,
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

    /*let mut conn = tcp::TcpConnection{
      state: tcp::TcpState::Listen,
      src_ip: [192, 168, 3, 1],
      dst_ip: [192, 168, 3, 2],
      src_port: 0,
      dst_port: 0,
      seq_num: 0,
      ack_num: 0,
      peer_seq_num: 0,
      };*/

    const MAX_CONNS: usize = 4;
    //let mut conns: [Option<tcp::TcpConnection>; MAX_CONNS] = Default::default();
    let mut conns: [Option<tcp::TcpConnection>; 4] = [None, None, None, None];

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
    let (syn, ack, fin, rst) = tcp::parse_flags(&tcp_header);

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

        send_response(conn, tcp::ACK, b"Connection established\r\n");
        return 1;

    } else if conn.state == tcp::TcpState::Established {
        if payload.len() > 0 {
            conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(payload.len() as u32);

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

    let mut tcp_len = 0;

    tcp_len = tcp::create_packet(
        conn.src_port,
        conn.dst_port,
        //u16::from_be(tcp_header.dest_port),
        //u16::from_be(tcp_header.source_port),
        conn.seq_num,
        conn.ack_num,
        flags,
        1024,
        payload,
        conn.src_ip,
        conn.dst_ip,
        //ipv4_header.dest_ip,
        //ipv4_header.source_ip,
        &mut out_buf
    );

    let ipv4_len = ipv4::create_packet(conn.dst_ip, conn.src_ip, 6, &out_buf[..tcp_len], &mut ipv4_buf);
    ipv4::send_packet(&ipv4_buf[..ipv4_len]);
}

