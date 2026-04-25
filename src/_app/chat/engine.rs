use crate::{
    net::ipv4::receive_loop_tcp,
    vga::write::{
        string,
    }
};

const MAX_PEERS: usize = 4;
const MAX_LINE_LEN: usize = 256;

static mut PEERS: [Option<Peer>; MAX_PEERS] = [None, None, None, None];
static mut LINE_BUF: [u8; MAX_LINE_LEN] = [0; MAX_LINE_LEN];
static mut LINE_LEN: usize = 0;

struct Peer {
    stream: TcpStream,
    recv_buf: [u8; MAX_LINE_LEN],
    recv_len: usize,
}

impl Peer {
    fn new(stream: TcpStream) -> Self {
        Peer {
            stream,
            recv_buf: [0; MAX_LINE_LEN],
            recv_len: 0,
        }
    }

    fn try_read_line(&mut self) -> Option<&str> {
        while let Some(byte) = self.stream.try_read_byte() {
            if self.recv_len < MAX_LINE_LEN {
                self.recv_buf[self.recv_len] = byte;
                self.recv_len += 1;
            }
            if byte == b'\n' {
                let msg = core::str::from_utf8(&self.recv_buf[..self.recv_len]).ok()?;
                self.recv_len = 0;
                return Some(msg);
            }
        }
        None
    }

    fn send_line(&mut self, line: &str) {
        let _ = self.stream.write_bytes(line.as_bytes());
    }
}

fn add_peer(p: Peer) {
    for slot in unsafe { PEERS.iter_mut() } {
        if slot.is_none() {
            *slot = Some(p);
            return;
        }
    }
}

fn broadcast_line(line: &str) {
    for peer in unsafe { PEERS.iter_mut() } {
        if let Some(p) = peer {
            p.send_line(line);
        }
    }
    print_line(line);
}


//
//
//

fn chat_main(my_port: u16, peer_ips: &[IpAddr]) {
    let listener = TcpListener::bind(my_port);

    // Connect to peers
    for &ip in peer_ips {
        if let Some(stream) = TcpStream::connect(ip, my_port) {
            add_peer(Peer::new(stream));
        }
    }

    loop {
        // Accept new incoming connections
        if let Some(new_stream) = listener.accept() {
            add_peer(Peer::new(new_stream));
        }

        unsafe {
            // Read from keyboard
            if let Some(ch) = keyboard_poll_char() {
                if ch == '\n' {
                    let line = unsafe { core::str::from_utf8_unchecked(&LINE_BUF[..LINE_LEN]) };
                    broadcast_line(line);
                    LINE_LEN = 0;
                } else if LINE_LEN < MAX_LINE_LEN {
                    LINE_BUF[LINE_LEN] = ch as u8;
                    LINE_LEN += 1;
                }
            }
        }

        // Check peers for incoming messages
        for peer in unsafe { PEERS.iter_mut() } {
            if let Some(p) = peer {
                if let Some(msg) = p.try_read_line() {
                    print_line(msg);
                    // Optional: relay to others
                    // broadcast_line(msg);
                }
            }
        }
    }
}

