use crate::acpi;
use crate::app;
use crate::init::config;
use crate::fs;
use crate::net;
use crate::sound;
use crate::time;
use crate::vga;

const KERNEL_VERSION: &[u8] = b"0.6.0";

struct Command {
    name: &'static [u8],
    description: &'static [u8],
    function: fn(args: &[u8], vga_index: &mut isize),
}

static COMMANDS: &[Command] = &[
    Command {
        name: b"beep",
        description: b"beeps",
        function: cmd_beep,
    },
    Command {
        name: b"cd",
        description: b"changes the current directory",
        function: cmd_cd,
    },
    Command {
        name: b"cls",
        description: b"clears the screen",
        function: cmd_clear,
    },
    Command {
        name: b"dir",
        description: b"lists the current directory",
        function: cmd_dir,
    },
    Command {
        name: b"echo",
        description: b"echos the arguments",
        function: cmd_echo,
    },
    Command {
        name: b"game",
        description: b"runs a simple VGA text mode game",
        function: cmd_game,
    },
    Command {
        name: b"help",
        description: b"shows this output",
        function: cmd_help,
    },
    Command {
        name: b"http",
        description: b"runs a simple HTTP/UDP handler",
        function: cmd_http,
    },
    Command {
        name: b"ping",
        description: b"pings the host over the serial line (ICMP/SLIP)",
        function: cmd_ping,
    },
    Command {
        name: b"response",
        description: b"waits for ICMP/SLIP request to come, then sends a response back",
        function: cmd_response,
    },
    Command {
        name: b"shutdown",
        description: b"shuts down the system",
        function: cmd_shutdown,
    },
    Command {
        name: b"tcp",
        description: b"tests the TCP implementation",
        function: cmd_tcp,
    },
    Command {
        name: b"time",
        description: b"prints system time and date",
        function: cmd_time,
    },
    Command {
        name: b"uptime",
        description: b"prints system uptime",
        function: cmd_uptime,
    },
    Command {
        name: b"version",
        description: b"prints the kernel version",
        function: cmd_version,
    }
];

pub fn handle(input: &[u8], vga_index: &mut isize) {
    // Only for strings!
    /*let mut parts = input.splitn(2, ' ');
      let cmd_name = parts.next().unwrap_or("");
      let cmd_args = parts.next().unwrap_or("");*/

    let (cmd_name, cmd_args) = split_cmd(input);

    match find_cmd(cmd_name) {
        Some(cmd) => {
            (cmd.function)(cmd_args, vga_index);
        }
        None => {
            if input.is_empty() {
                return;
            }

            // Echo back the input
            vga::write::string(vga_index, b"unknown command: ", vga::buffer::Color::Red);
            vga::write::string(vga_index, cmd_name, vga::buffer::Color::White);
            vga::write::newline(vga_index);
        }
    }
}

//
//  HELPER FUNCTIONS
//

#[allow(clippy::manual_find)]
fn find_cmd(name: &[u8]) -> Option<&'static Command> {
    for cmd in COMMANDS {
        if cmd.name == name {
            return Some(cmd);
        }
    }
    None
}

fn split_cmd(input: &[u8]) -> (&[u8], &[u8]) {
    // Find the first space
    if let Some(pos) = input.iter().position(|&c| c == b' ') {
        let (cmd, args) = input.split_at(pos);
        // skip the space character for args
        let args_slice = args.get(1..).unwrap_or(&[]);
        (cmd, args_slice)
    } else {
        // No space found, entire input is the command
        (input, &[])
    }
}

fn to_uppercase_ascii(input: &mut [u8]) {
    for byte in input.iter_mut() {
        if *byte >= b'a' && *byte <= b'z' {
            *byte -= 32;
        }
    }
}

//
//  COMMAND FUNCTIONS
//

fn cmd_beep(_args: &[u8], _vga_index: &mut isize) {
    sound::beep::beep(5000);

    for _ in 0..3_000_000 {
        unsafe { core::arch::asm!("nop"); }
    }

    sound::beep::stop_beep();
}

fn cmd_cd(args: &[u8], vga_index: &mut isize) {
    let floppy = fs::block::Floppy;

    if args.len() == 0 {
        unsafe {
            config::PATH_CLUSTER = 0;
            config::set_path(b"/");
        }
        return;
    }

    match fs::fs::Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            unsafe {
                let cluster = fs.list_dir(config::PATH_CLUSTER, args, vga_index);

                if cluster > 0 {
                    config::PATH_CLUSTER = cluster as u16;
                    config::set_path(args);
                }
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}

fn cmd_clear(_args: &[u8], vga_index: &mut isize) {
    vga::screen::clear(vga_index);
}

fn cmd_dir(_args: &[u8], vga_index: &mut isize) {
    let floppy = fs::block::Floppy;

    match fs::fs::Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            unsafe {
                fs.list_dir(config::PATH_CLUSTER, &[], vga_index);
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}

fn cmd_echo(args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, args, vga::buffer::Color::White);
    vga::write::newline(vga_index);
}

fn cmd_help(_args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, b"List of commands:", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    for cmd in COMMANDS {
        vga::write::string(vga_index, b" ", vga::buffer::Color::Blue);
        vga::write::string(vga_index, cmd.name, vga::buffer::Color::Blue);
        vga::write::string(vga_index, b": ", vga::buffer::Color::Blue);
        vga::write::string(vga_index, cmd.description, vga::buffer::Color::White);
        vga::write::newline(vga_index);
    }
}

fn cmd_game(_args: &[u8], vga_index: &mut isize) {
    vga::screen::clear(vga_index);
    app::game::run(vga_index);
}

fn cmd_http(_args: &[u8], vga_index: &mut isize) {
    fn callback(packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = net::ipv4::parse_packet(packet) {
            if ipv4_header.protocol != 17 {
                return 1;
            }

            return app::http_server::udp_handler(&ipv4_header, ipv4_payload);
        }
        0
    }

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Starting a simple HTTP/UDP handler (hit any key to interrupt)...", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    loop {
        let ret = net::ipv4::receive_loop(callback);

        if ret == 0 {
            vga::write::string(vga_index, b"Received a HTTP request, sending response", vga::buffer::Color::White);
            vga::write::newline(vga_index);
        } else if ret == 3 {
            vga::write::string(vga_index, b"Keyboard interrupt", vga::buffer::Color::White);
            vga::write::newline(vga_index);
            break;
        }
    }
}

fn cmd_ping(_args: &[u8], vga_index: &mut isize) {
    let src_ip = [192, 168, 3, 2];
    let dst_ip = [192, 168, 3, 1];
    let protocol = 1;
    let identifier = 1342;
    let sequence_no = 1;
    let payload = b"ping from r2"; // optional payload*/

    let mut icmp_buf = [0u8; 256];
    let mut ipv4_buf = [0u8; 1500];

    // Create ICMP packet and encapsulate it in the IPv4 packet.
    let icmp_len = net::icmp::create_packet(8, identifier, sequence_no, payload, &mut icmp_buf);
    let icmp_slice = icmp_buf.get(..icmp_len).unwrap_or(&[]);

    let ipv4_len = net::ipv4::create_packet(src_ip, dst_ip, protocol, icmp_slice, &mut ipv4_buf);
    let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);

    vga::write::string(vga_index, b"Sending a ping packet...", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    net::ipv4::send_packet(ipv4_slice);
}

fn cmd_response(_args: &[u8], vga_index: &mut isize) {
    fn callback(packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = net::ipv4::parse_packet(packet) {
            if ipv4_header.protocol != 1 {
                return 1;
            }

            if let Some((icmp_header, icmp_payload)) = net::icmp::parse_packet(ipv4_payload) {
                if icmp_header.icmp_type != 8 {
                    return 2;
                }

                let mut icmp_buf = [0u8; 64];
                let mut ipv4_buf = [0u8; 1500];

                let icmp_len = net::icmp::create_packet(0, icmp_header.identifier, icmp_header.sequence_number, icmp_payload, &mut icmp_buf);
                let icmp_slice = icmp_buf.get(..icmp_len).unwrap_or(&[]);

                //let ipv4_len = net::ipv4::create_packet(ipv4_header.dest_ip, ipv4_header.source_ip, ipv4_header.protocol, &icmp_buf[..icmp_len], &mut ipv4_buf);
                let ipv4_len = net::ipv4::create_packet([192, 168, 3, 2], ipv4_header.source_ip, ipv4_header.protocol, icmp_slice, &mut ipv4_buf);
                let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);

                net::ipv4::send_packet(ipv4_slice);
            }
        }
        0
    }

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Waiting for an ICMP echo request (hit any key to interrupt)...", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    loop {
        let ret = net::ipv4::receive_loop(callback);

        if ret == 0 {
            vga::write::string(vga_index, b"Received a ping request, sending a response", vga::buffer::Color::White);
            vga::write::newline(vga_index);
            /*} else if ret == 1 {
              vga::write::string(vga_index, b"Wrong IPv4 protocol (not ICMP) received", vga::buffer::Color::Green);
              vga::write::newline(vga_index);*/
    } else if ret == 2 {
        vga::write::string(vga_index, b"Received a non-request ICMP packet", vga::buffer::Color::Green);
        vga::write::newline(vga_index);
    } else if ret == 3 {
        vga::write::string(vga_index, b"Keyboard interrupt", vga::buffer::Color::White);
        vga::write::newline(vga_index);
        break;
    }
    }
}

fn cmd_shutdown(_args: &[u8], vga_index: &mut isize) {
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b" --- Shutting down", vga::buffer::Color::Cyan);

    for _ in 0..3 {
        for _ in 0..3_500_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
        vga::write::string(vga_index, b". ", vga::buffer::Color::Cyan);
    }

    acpi::shutdown::shutdown();
}

fn cmd_tcp(_args: &[u8], vga_index: &mut isize) {
    app::tcp_handler::handle(vga_index);
}

fn cmd_time(_args: &[u8], vga_index: &mut isize) {
    let (y, mo, d, h, m, s) = time::rtc::read_rtc_full();

    vga::write::string(vga_index, b"RTC Time: ", vga::buffer::Color::White);
    vga::write::number(vga_index, h as u64);

    vga::write::string(vga_index, b":", vga::buffer::Color::White);

    if m < 10 { 
        vga::write::string(vga_index, b"0", vga::buffer::Color::White); 
    }
    vga::write::number(vga_index, m as u64);

    vga::write::string(vga_index, b":", vga::buffer::Color::White);

    if s < 10 { 
        vga::write::string(vga_index, b"0", vga::buffer::Color::White); 
    }
    vga::write::number(vga_index, s as u64);

    vga::write::newline(vga_index);

    vga::write::string(vga_index, b"RTC Date: ", vga::buffer::Color::White);

    if d < 10 {
        vga::write::string(vga_index, b"0", vga::buffer::Color::White); 
    }
    vga::write::number(vga_index, d as u64);
    vga::write::string(vga_index, b"-", vga::buffer::Color::White);

    if mo < 10 {
        vga::write::string(vga_index, b"0", vga::buffer::Color::White); 
    }
    vga::write::number(vga_index, mo as u64);
    vga::write::string(vga_index, b"-", vga::buffer::Color::White);

    vga::write::number(vga_index, y as u64);

    vga::write::newline(vga_index);
}

fn cmd_uptime(_args: &[u8], vga_index: &mut isize) {
    let total_seconds = time::acpi::get_uptime_seconds();

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    // Print formatted
    vga::write::string(vga_index, b"Uptime: ", vga::buffer::Color::White);
    vga::write::number(vga_index, hours);
    vga::write::string(vga_index, b":", vga::buffer::Color::White);

    if minutes < 10 {
        vga::write::string(vga_index, b"0", vga::buffer::Color::White);
    }

    vga::write::number(vga_index, minutes);
    vga::write::string(vga_index, b":", vga::buffer::Color::White);

    if seconds < 10 {
        vga::write::string(vga_index, b"0", vga::buffer::Color::White);
    }

    vga::write::number(vga_index, seconds);

    vga::write::newline(vga_index);
}

fn cmd_version(_args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, b"Version: ", vga::buffer::Color::White);
    vga::write::string(vga_index, KERNEL_VERSION, vga::buffer::Color::White);
    vga::write::newline(vga_index);
}
