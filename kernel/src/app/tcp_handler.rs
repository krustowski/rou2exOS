use crate::net::ipv4;
use crate::net::tcp;
use crate::vga;

pub fn handle_tcp_packet(conn: &mut tcp::TcpConnection, tcp_header: &tcp::TcpHeader, payload: &[u8]) {
    let flags = u16::from_be(tcp_header.data_offset_reserved_flags) & 0x01FF;

    match conn.state {
        tcp::TcpState::Listen => {
            if flags & tcp::SYN != 0 {
                conn.state = tcp::TcpState::SynReceived;
                conn.seq_num = 0x1000; // random ISN
                conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(1);

                send_tcp(
                    conn.src_ip,
                    conn.dst_ip,
                    conn.src_port,
                    conn.dst_port,
                    conn.seq_num,
                    conn.ack_num,
                    tcp::SYN | tcp::ACK,
                    &[],
                );
                conn.seq_num += 1;
            }
        }

        tcp::TcpState::SynReceived => {
            if flags & tcp::ACK != 0 {
                conn.state = tcp::TcpState::Established;
            }
        }

        tcp::TcpState::Established => {
            if payload.len() > 0 {
                let seq = u32::from_be(tcp_header.seq_num);
                conn.ack_num = seq.wrapping_add(payload.len() as u32);
                send_tcp(
                    conn.src_ip,
                    conn.dst_ip,
                    conn.src_port,
                    conn.dst_port,
                    conn.seq_num,
                    conn.ack_num,
                    tcp::ACK,
                    &[],
                );

                // echo back?
                send_tcp(
                    conn.src_ip,
                    conn.dst_ip,
                    conn.src_port,
                    conn.dst_port,
                    conn.seq_num,
                    conn.ack_num,
                    tcp::PSH | tcp::ACK,
                    b"Hello from rou2exOS!",
                );
                conn.seq_num += b"Hello from rou2exOS!".len() as u32;
            }

            if flags & tcp::FIN != 0 {
                conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(1);
                send_tcp(
                    conn.src_ip,
                    conn.dst_ip,
                    conn.src_port,
                    conn.dst_port,
                    conn.seq_num,
                    conn.ack_num,
                    tcp::ACK,
                    &[],
                );
                conn.state = tcp::TcpState::CloseWait;
            }
        }

        tcp::TcpState::CloseWait => {
            send_tcp(
                conn.src_ip,
                conn.dst_ip,
                conn.src_port,
                conn.dst_port,
                conn.seq_num,
                conn.ack_num,
                tcp::FIN | tcp::ACK,
                &[],
            );
            conn.seq_num += 1;
            conn.state = tcp::TcpState::LastAck;
        }

        tcp::TcpState::LastAck => {
            if flags & tcp::ACK != 0 {
                conn.state = tcp::TcpState::Closed;
            }
        }

        _ => {}
    }
}

fn send_tcp(
    src_ip: [u8; 4], 
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    seq_num: u32,
    ack_num: u32,
    flags: u16,
    payload: &[u8],
) {
    let mut out_buf = [0u8; 500];
    let mut ipv4_buf = [0u8; 1500];

    let tcp_len = tcp::create_packet(
        src_port,
        dst_port,
        seq_num,
        ack_num,
        flags,
        1024,
        payload,
        dst_ip,
        src_ip,
        &mut out_buf,
    );

    let ipv4_len = ipv4::create_packet([192, 168, 3, 2], [192, 168, 3, 1], 6, &out_buf[20..20 + tcp_len], &mut ipv4_buf);

    ipv4::send_packet(&ipv4_buf[..ipv4_len]);

}

//
//
//

pub fn handle(vga_index: &mut isize) {
    fn callback(conn: &mut tcp::TcpConnection, packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = ipv4::parse_packet(packet) {
            if let Some((tcp_header, payload)) = tcp::parse_packet(ipv4_payload) {
                let (syn, ack, fin, rst) = tcp::parse_flags(&tcp_header);

                let mut out_buf = [0u8; 500];
                let mut ipv4_buf = [0u8; 1500];

                let mut tcp_len = 0;

                //if flags & 0x002 == 0x002 {
                if syn && !ack {
                    conn.src_port = u16::from_be(tcp_header.dest_port);
                    conn.dst_port = u16::from_be(tcp_header.source_port);

                    // SYN received, reply with SYN+ACK
                    conn.state = tcp::TcpState::SynReceived;
                    conn.seq_num = 1;
                    conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(1);

                    send_response(conn, tcp::SYN | tcp::ACK, payload);

                    return 0;

                } else if ack && conn.state == tcp::TcpState::SynReceived {
                    // Ready to receive/send data
                    conn.state = tcp::TcpState::Established;
                    conn.seq_num += 1;
                    conn.ack_num = u32::from_be(tcp_header.seq_num);

                    send_response(conn, tcp::ACK, b"Ale vitaj ne\r\n");

                    return 1;
                }
            }
        }
        2
    }

    let mut conn = tcp::TcpConnection{
        state: tcp::TcpState::Listen,
        src_ip: [192, 168, 3, 1],
        dst_ip: [192, 168, 3, 2],
        src_port: 0,
        dst_port: 0,
        seq_num: 0,
        ack_num: 0,
        peer_seq_num: 0,
    };

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Starting a simple TCP tester (hit any key to interrupt)...", 0x0f);
    vga::write::newline(vga_index);

    loop {
        let ret = ipv4::receive_loop_tcp(&mut conn, callback);

        if ret == 0 {
            vga::write::string(vga_index, b"Received SYN", 0x0f);
            vga::write::newline(vga_index);

        } else if ret == 1 {
            vga::write::string(vga_index, b"Received ACK", 0x0f);
            vga::write::newline(vga_index);
        } else if ret == 3 {
            vga::write::string(vga_index, b"Keyboard interrupt", 0x0f);
            vga::write::newline(vga_index);
            break;
        }
    }
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

