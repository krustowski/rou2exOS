use crate::acpi;
use crate::net;
use crate::sound;
use crate::time;
use crate::vga;

const KERNEL_VERSION: &[u8] = b"0.3.0";

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
        name: b"cls",
        description: b"clears the screen",
        function: cmd_clear,
    },
    Command {
        name: b"echo",
        description: b"echos the arguments",
        function: cmd_echo,
    },
    Command {
        name: b"help",
        description: b"shows this output",
        function: cmd_help,
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
            if input.len() == 0 {
                return;
            }

            // Echo back the input
            vga::write::string(vga_index, b"unknown command: ", 0xc);
            vga::write::string(vga_index, cmd_name, 0x0f);
            vga::write::newline(vga_index);
        }
    }
}

//
//  HELPER FUNCTIONS
//

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
        (cmd, &args[1..])
    } else {
        // No space found, entire input is the command
        (input, &[])
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

fn cmd_clear(_args: &[u8], vga_index: &mut isize) {
    vga::screen::clear(vga_index);
}

fn cmd_echo(args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, args, 0x0f);
    vga::write::newline(vga_index);
}

fn cmd_help(_args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, b"List of commands:", 0x0f);
    vga::write::newline(vga_index);

    for cmd in COMMANDS {
        vga::write::string(vga_index, b" - ", 0x09);
        vga::write::string(vga_index, cmd.name, 0x09);
        vga::write::string(vga_index, b": ", 0x09);
        vga::write::string(vga_index, cmd.description, 0x0f);
        vga::write::newline(vga_index);
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
    let ipv4_len = net::ipv4::create_packet(src_ip, dst_ip, protocol, &icmp_buf[..icmp_len], &mut ipv4_buf);

    vga::write::string(vga_index, b"Sending a ping packet...", 0x0f);
    vga::write::newline(vga_index);

    net::ipv4::send_packet(&ipv4_buf[..ipv4_len]);
}

fn cmd_response(_args: &[u8], vga_index: &mut isize) {
    fn callback(packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = net::ipv4::parse_packet(packet) {
            if ipv4_header.protocol != 1 {
                return 1;
            }

            if let Some((icmp_header, icmp_payload)) = net::icmp::parse_packet(&ipv4_payload) {
                if icmp_header.icmp_type != 8 {
                    return 2;
                }

                let mut icmp_buf = [0u8; 64];
                let mut ipv4_buf = [0u8; 1500];

                let icmp_len = net::icmp::create_packet(0, icmp_header.identifier, icmp_header.sequence_number, icmp_payload, &mut icmp_buf);
                let ipv4_len = net::ipv4::create_packet([192, 168, 3, 2], ipv4_header.source_ip, ipv4_header.protocol, &icmp_buf[..icmp_len], &mut ipv4_buf);

                net::ipv4::send_packet(&ipv4_buf[..ipv4_len]);
            }
        }
        0
    }

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Waiting for an ICMP echo request...", 0x0f);
    vga::write::newline(vga_index);

    loop {
        let ret = net::ipv4::receive_loop(callback);

        if ret == 0 {
            vga::write::string(vga_index, b"Received a ping request, sending response...", 0x0f);
            vga::write::newline(vga_index);
            break;
        } else if ret == 1 {
            vga::write::string(vga_index, b"Wrong IPv4 protocol (not ICMP) received", 0xc);
            vga::write::newline(vga_index);
        } else {
            vga::write::string(vga_index, b"Received a non-request ICMP packet", 0xc);
            vga::write::newline(vga_index);
        }
    }
}

fn cmd_shutdown(_args: &[u8], vga_index: &mut isize) {
    vga::write::newline(vga_index);
    vga::write::string(vga_index, b" --- Shutting down", 0xb);

    for _ in 0..3 {
        for _ in 0..3_500_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
        vga::write::string(vga_index, b".", 0xb);
    }

    acpi::shutdown::shutdown();
}

fn cmd_time(_args: &[u8], vga_index: &mut isize) {
    let (y, mo, d, h, m, s) = time::rtc::read_rtc_full();

    vga::write::string(vga_index, b"RTC Time: ", 0x0f);
    vga::write::number(vga_index, &mut (h as u64));

    vga::write::string(vga_index, b":", 0x0f);

    if m < 10 { 
        vga::write::string(vga_index, b"0", 0x0f); 
    }
    vga::write::number(vga_index, &mut (m as u64));

    vga::write::string(vga_index, b":", 0x0f);

    if s < 10 { 
        vga::write::string(vga_index, b"0", 0x0f); 
    }
    vga::write::number(vga_index, &mut (s as u64));

    vga::write::newline(vga_index);

    vga::write::string(vga_index, b"RTC Date: ", 0x0f);

    if d < 10 {
        vga::write::string(vga_index, b"0", 0x0f); 
    }
    vga::write::number(vga_index, &mut (d as u64));
    vga::write::string(vga_index, b"-", 0x0f);

    if mo < 10 {
        vga::write::string(vga_index, b"0", 0x0f); 
    }
    vga::write::number(vga_index, &mut (mo as u64));
    vga::write::string(vga_index, b"-", 0x0f);

    vga::write::number(vga_index, &mut (y as u64));

    vga::write::newline(vga_index);
}

fn cmd_uptime(_args: &[u8], vga_index: &mut isize) {
    let total_seconds = time::acpi::get_uptime_seconds();

    let mut hours = total_seconds / 3600;
    let mut minutes = (total_seconds % 3600) / 60;
    let mut seconds = total_seconds % 60;

    // Print formatted
    vga::write::string(vga_index, b"Uptime: ", 0x0f);
    vga::write::number(vga_index, &mut hours);
    vga::write::string(vga_index, b":", 0x0f);

    if minutes < 10 {
        vga::write::string(vga_index, b"0", 0x0f);
    }

    vga::write::number(vga_index, &mut minutes);
    vga::write::string(vga_index, b":", 0x0f);

    if seconds < 10 {
        vga::write::string(vga_index, b"0", 0x0f);
    }

    vga::write::number(vga_index, &mut seconds);

    vga::write::newline(vga_index);
}

fn cmd_version(_args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, b"Version: ", 0x0f);
    vga::write::string(vga_index, KERNEL_VERSION, 0x0f);
    vga::write::newline(vga_index);
}
