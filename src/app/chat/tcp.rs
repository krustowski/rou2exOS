use crate::{
    input::{
        keyboard::{self, move_cursor_index},
        port,
    },
    net::{
        ipv4, 
        serial,
        slip,
        tcp::{self, TcpConnection},
    },
    vga::{
        buffer::Color, 
        screen::{clear, scroll_at}, 
        write::{
            byte,
            newline,
            string,
        }
    },
};

#[derive(PartialEq, PartialOrd)]
pub enum HandleState {
    ConnOK,
    //
    GotSYN,
    GotACK,
    GotFIN,
    //
    SendResponse,
    KeyboardInterrupt,
    //
    UnknownConn,
    FreedSocket,
    NoFreeSockets,
    //
    EmptySlot,
}

const END_LINE: isize = 2 * (80 * 24 + 1);

static mut RESPONSE_BUFFER: [u8; 256] = [0u8; 256];
static mut RESPONSE_LENGTH: usize = 0;

static mut VGA_INDEX: isize = 0;
static mut VGA_INDEX_WRITE: isize = END_LINE;

pub fn receive_loop_tcp(
    conns: &mut [Option<tcp::TcpConnection>; ipv4::MAX_CONNS],
    callback: fn(conns: &mut [Option<tcp::TcpConnection>; ipv4::MAX_CONNS], packet: &[u8]) -> HandleState,
) -> HandleState {
    let mut temp_buf: [u8; 2048] = [0; 2048];
    let mut packet_buf: [u8; 2048] = [0; 2048];
    let mut temp_len: usize = 0;

    serial::init();

    loop {
        // While the keyboard is idle...
        while port::read(0x64) & 1 == 0 {
            if serial::ready() && temp_len <= temp_buf.len() {
                if let Some(p) = temp_buf.get_mut(temp_len) {
                    *p = serial::read();
                }
                temp_len += 1;

                let temp_slice = temp_buf.get(..temp_len).unwrap_or(&[]);

                if let Some(packet_len) = slip::decode(temp_slice, &mut packet_buf) {
                    // Full packet decoded
                    let packet_slice = packet_buf.get(..packet_len).unwrap_or(&[]);
                    return callback(conns, packet_slice);
                }
            }
        }

        let key = port::read(0x60);
        if key & 0x80 == 0 {
            match key {
                0x01 => {
                    // Send DISCONNECT + exit
                    break;
                }
                // Enter
                0x1C => {
                    // Send message
                    unsafe {
                        for i in 0..RESPONSE_LENGTH {
                            VGA_INDEX_WRITE -= 2;
                            byte(&mut VGA_INDEX_WRITE, b' ', Color::Black);
                            VGA_INDEX_WRITE -= 2;
                        }

                        move_cursor_index(&mut END_LINE);
                        return HandleState::SendResponse;
                    }
                }
                // Backspace
                0x0E => {
                    unsafe {
                        if RESPONSE_LENGTH > 0 {
                            RESPONSE_LENGTH -= 1;

                            VGA_INDEX_WRITE -= 2;
                            byte(&mut VGA_INDEX_WRITE, b' ', Color::Black);
                            VGA_INDEX_WRITE -= 2;
                            move_cursor_index(&mut VGA_INDEX_WRITE);
                        }
                    }
                }
                _ => {
                    if let Some(ascii) = keyboard::scancode_to_ascii(key) {
                        unsafe {
                            if RESPONSE_LENGTH < 256 {
                                if let Some(w) = RESPONSE_BUFFER.get_mut(RESPONSE_LENGTH) {
                                    *w = ascii;
                                    RESPONSE_LENGTH += 1;

                                    byte(&mut VGA_INDEX_WRITE, ascii, Color::Yellow);
                                    move_cursor_index(&mut VGA_INDEX_WRITE);
                                    //scroll(&mut VGA_INDEX);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    HandleState::KeyboardInterrupt
}

fn filter_and_close_conns(conns: &mut [Option<tcp::TcpConnection>; ipv4::MAX_CONNS]) -> HandleState {
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
                        return HandleState::FreedSocket;
            }
        }
    }
    HandleState::ConnOK
}

pub fn handle_conns(vga_index: &mut isize, ips: &[[u8; 4]; 4]) {
    fn callback(conns: &mut [Option<tcp::TcpConnection>; ipv4::MAX_CONNS], packet: &[u8]) -> HandleState {
        if let Some((ipv4_header, ipv4_payload)) = ipv4::parse_packet(packet) {
            if let Some((tcp_header, payload)) = tcp::parse_packet(ipv4_payload) {
                let (syn, ack, _fin, _rst) = tcp::parse_flags(&tcp_header);

                // Parse IP and port
                let src_ip = ipv4_header.source_ip;
                let dst_ip = ipv4_header.dest_ip;
                let src_port = u16::from_be(tcp_header.source_port);
                let dst_port = u16::from_be(tcp_header.dest_port);

                // Loop over conns and free closed ones
                let conn_state = filter_and_close_conns(conns);
                if conn_state > HandleState::ConnOK {
                    return conn_state;
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
                        // Unexpected: maybe_existing was Some, but inner value was None
                        None => return HandleState::FreedSocket, 
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
                                None => return HandleState::EmptySlot, // This shouldn't happen, just inserted Some
                            }
                        }
                        None => return HandleState::NoFreeSockets, // No free slot — drop the packet
                    }
                } else {
                    // Packet for unknown connection — drop
                    return HandleState::UnknownConn;
                };

                return handle_tcp_packet(conn, &tcp_header, payload);
            }
        }
        HandleState::ConnOK
    }

    let mut conns: [Option<tcp::TcpConnection>; ipv4::MAX_CONNS] = Default::default();

    if ips.len() > 1 && ips[0][0] != 0 {
        let mut new_conn = tcp::TcpConnection {
            state: tcp::TcpState::Listen,
            src_ip: ips[0],
            dst_ip: ips[1],
            src_port: 33333,
            dst_port: 12345,
            seq_num: 0x1000,
            ack_num: 0,
        };

        send_response(&mut new_conn, tcp::SYN, &[]);
        
        unsafe {
            string(&mut VGA_INDEX, b"[NEW] Dialing....", Color::Cyan);
            newline(&mut VGA_INDEX);
        }

        conns[0] = Some(new_conn);
    }

    unsafe {
        VGA_INDEX = 0;
    }

    move_cursor_index(&mut END_LINE);

    loop {
        let ret = receive_loop_tcp(&mut conns, callback);

        unsafe {
            match ret {
                HandleState::GotSYN => {
                    //string(vga_index, b"Received SYN", Color::White);
                }
                HandleState::GotACK => {
                    //string(vga_index, b"Received ACK", Color::White);
                }
                HandleState::GotFIN => {
                    //string(vga_index, b"Received FIN", Color::White);
                    string(&mut VGA_INDEX, b"[FIN] Client disconnected.", Color::White);
                    newline(&mut VGA_INDEX);
                }
                HandleState::KeyboardInterrupt => {
                    //string(&mut VGA_INDEX, b"Keyboard interrupt", Color::White);
                    //newline(&mut VGA_INDEX);

                    let found_conn = conns.iter_mut().find(|entry| {
                        if let Some(conn) = entry {
                            conn.src_port == 12345 || conn.dst_port == 12345
                        } else {
                            false
                        }
                    });

                    let conn = if let Some(slot) = found_conn {
                        match slot.as_mut() {
                            Some(c) => c,
                            // Unexpected: maybe_existing was Some, but inner value was None
                            None => {
                                continue;
                            }
                        }
                    } else { 
                        continue; 
                    };

                    send_response(conn, tcp::ACK | tcp::PSH | tcp::FIN, b"DISCONNECT\n");
                    conn.state = tcp::TcpState::Closed;
                    break;
                }
                HandleState::FreedSocket => {
                    //string(vga_index, b"Freed socket", Color::White);
                }
                HandleState::UnknownConn => {
                    //string(vga_index, b"Unknown conn", Color::White);
                }
                HandleState::NoFreeSockets => {
                    //string(vga_index, b"No free slots", Color::White);
                }
                HandleState::SendResponse => {
                    let found_conn = for_each_conn(&mut conns, |conn| {
                        if conn.src_port == 12345 || conn.dst_port == 12345 {
                            Some(conn)
                        } else {
                            None
                        }
                    });

                    if let Some(conn) = found_conn {
                        if let Some(response) = RESPONSE_BUFFER.get_mut(..RESPONSE_LENGTH + 1) {
                            if let Some(b) = response.get_mut(response.len() - 1) {
                                *b = b'\n';
                            }

                            send_response(conn, tcp::ACK | tcp::PSH, response);
                            conn.seq_num += (RESPONSE_LENGTH + 1) as u32;

                            if let Some(b) = response.get_mut(response.len() - 1) {
                                *b = b' ';
                            }

                            string(&mut VGA_INDEX, b"[you]: ", Color::Yellow);
                            string(&mut VGA_INDEX, &response, Color::White);
                            newline(&mut VGA_INDEX);
                            scroll_at(&mut VGA_INDEX, &mut 24);
                        }
                    }

                    RESPONSE_BUFFER = [0u8; 256];
                    RESPONSE_LENGTH = 0;
                }
                _ => {}
            }
        }
    }
    clear(vga_index);
}

fn for_each_conn<'a, F, R>(conns: &'a mut [Option<TcpConnection>], mut f: F) -> Option<R> where F: FnMut(&'a mut TcpConnection) -> Option<R> {
    for conn in conns.iter_mut() {
        if let Some(ref mut c) = conn {
            if let Some(result) = f(c) {
                return Some(result);
            }
        }
    }
    None
}

fn handle_tcp_packet(conn: &mut tcp::TcpConnection, tcp_header: &tcp::TcpHeader, payload: &[u8]) -> HandleState {
    let (syn, ack, fin, _rst) = tcp::parse_flags(tcp_header);

    if syn && !ack {
        conn.src_port = u16::from_be(tcp_header.dest_port);
        conn.dst_port = u16::from_be(tcp_header.source_port);

        // SYN received, reply with SYN+ACK
        conn.state = tcp::TcpState::SynReceived;
        conn.seq_num = 1;
        conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(1);

        unsafe {
            if conn.src_port == 12345 {
                string(&mut VGA_INDEX, b"[NEW] New connection.", Color::Cyan);
                newline(&mut VGA_INDEX);
            }
        }

        send_response(conn, tcp::SYN | tcp::ACK, payload);
        return HandleState::GotSYN;
    } 

    if ack && conn.state == tcp::TcpState::SynReceived {
        // Ready to receive/send data
        conn.state = tcp::TcpState::Established;
        conn.ack_num = u32::from_be(tcp_header.seq_num);
        conn.seq_num += 1;

        //send_response(conn, tcp::ACK, b"Connection established\r\n");
        send_response(conn, tcp::ACK, &[]);
        return HandleState::GotACK;

    } else if conn.state == tcp::TcpState::Established && (conn.src_port == 12345 || conn.dst_port == 12345) {
        if !payload.is_empty() {

            // Send ACK to received message
            conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(payload.len() as u32);
            send_response(conn, tcp::ACK, &[]);

            // Close the conn if the client wants to disconnect
            if payload.starts_with(b"DISCONNECT") {
                conn.ack_num = u32::from_be(tcp_header.seq_num).wrapping_add(payload.len() as u32);
                send_response(conn, tcp::ACK | tcp::PSH | tcp::FIN, b"BYE\n");
                conn.state = tcp::TcpState::Closed;

                return HandleState::GotFIN;
            }

            // Write the message out
            unsafe {
                let mut crt = 0;

                if let Some(end) = payload.get(payload.len() - 1) {
                    if *end == b'\n' {
                        crt = 1;
                    }
                }

                if let Some(msg) = payload.get(..payload.len() - 1 - crt) {
                    string(&mut VGA_INDEX, b"[peer]: ", Color::Green);
                    string(&mut VGA_INDEX, msg, Color::White);
                    newline(&mut VGA_INDEX);
                    scroll_at(&mut VGA_INDEX, &mut 24);
                }
            }
        } else {
            if fin {
                // Close the connection
                conn.state = tcp::TcpState::Closed;
                conn.ack_num = u32::from_be(tcp_header.seq_num);

                send_response(conn, tcp::FIN | tcp::ACK, &[]);
                return HandleState::GotFIN;
            }

            // Just ACK to keep connection alive
            conn.ack_num = u32::from_be(tcp_header.seq_num);
            send_response(conn, tcp::ACK, &[]);
        }
    }
    HandleState::GotACK
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

