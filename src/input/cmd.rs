use crate::acpi;
use crate::app;
use crate::audio;
use crate::init::config;
use crate::fs::fat12::{block::Floppy, fs::Fs, check::run_check};
use crate::init::config::PATH_CLUSTER;
use crate::net;
use crate::time;
use crate::vga;
use crate::vga::write::newline;
use crate::input::keyboard;
use crate::tui::{widget::{Container, Window, Label}, app::TuiApp};

const KERNEL_VERSION: &[u8] = b"0.7.2";

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
        name: b"chat",
        description: b"starts a chat",
        function: cmd_chat,
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
        name: b"ed",
        description: b"runs a minimalistic text editor",
        function: cmd_ed,
    },
    Command {
        name: b"ether",
        description: b"runs the Ethernet frame handler",
        function: cmd_ether,
    },
    Command {
        name: b"fsck",
        description: b"runs the filesystem check",
        function: cmd_fsck,
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
        name: b"menu",
        description: b"renders a sample menu",
        function: cmd_menu,
    },
    Command {
        name: b"mkdir",
        description: b"creates a subdirectory",
        function: cmd_mkdir,
    },
    Command {
        name: b"mv",
        description: b"renames a file",
        function: cmd_mv,
    },
    Command {
        name: b"ping",
        description: b"pings the host over the serial line (ICMP/SLIP)",
        function: cmd_ping,
    },
    Command {
        name: b"read",
        description: b"prints the output of a file",
        function: cmd_read,
    },
    Command {
        name: b"response",
        description: b"waits for ICMP/SLIP request to come, then sends a response back",
        function: cmd_response,
    },
    Command {
        name: b"rm",
        description: b"removes a file",
        function: cmd_rm,
    },
    Command {
        name: b"shutdown",
        description: b"shuts down the system",
        function: cmd_shutdown,
    },
    Command {
        name: b"snake",
        description: b"runs a simple VGA text mode snake-like game",
        function: cmd_snake,
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
    },
    Command {
        name: b"write",
        description: b"writes arguments to a sample file on floppy",
        function: cmd_write,
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

pub fn split_cmd(input: &[u8]) -> (&[u8], &[u8]) {
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

const MAX_IPS: usize = 4;

pub fn parse_ip_args(input: &[u8], out: &mut [[u8; 4]; MAX_IPS]) -> usize {
    let mut ip_count = 0;
    let mut i = 0;
    let len = input.len();

    while i < len && ip_count < MAX_IPS {
        let mut ip = [0u8; 4];
        let mut octet = 0;
        let mut val = 0u16;
        let mut digit_seen = false;

        while i < len {
            match input[i] {
                b'0'..=b'9' => {
                    val = val * 10 + (input[i] - b'0') as u16;
                    if val > 255 {
                        break;
                    }
                    digit_seen = true;
                }
                b'.' => {
                    if !digit_seen || octet >= 3 {
                        break;
                    }
                    ip[octet] = val as u8;
                    octet += 1;
                    val = 0;
                    digit_seen = false;
                }
                b' ' => {
                    i += 1;
                    break;
                }
                _ => {
                    break;
                }
            }
            i += 1;
        }

        if digit_seen && octet == 3 {
            ip[3] = val as u8;
            out[ip_count] = ip;
            ip_count += 1;
        }

        while i < len && input[i] == b' ' {
            i += 1;
        }
    }

    ip_count
}

pub fn to_uppercase_ascii(input: &mut [u8; 11]) {
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
    //sound::beep::beep(5000);
    audio::midi::play_melody();

    for _ in 0..3_000_000 {
        unsafe { core::arch::asm!("nop"); }
    }

    audio::beep::stop_beep();
}

fn cmd_cd(args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    if args.len() == 0 || args.len() > 11 {
        unsafe {
            config::PATH_CLUSTER = 0;
            config::set_path(b"/");
        }
        return;
    }

    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.len() == 0 || filename_input.len() > 12 {
        vga::write::string(vga_index, b"Usage: ed <filename>", vga::buffer::Color::Yellow);
        newline(vga_index);
        return;
    }

    let mut filename = [b' '; 12];
    if let Some(slice) = filename.get_mut(..filename_input.len()) {
        slice.copy_from_slice(filename_input);
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut cluster: u16 = 0;

            unsafe {
                fs.for_each_entry(config::PATH_CLUSTER, |entry| {
                    if entry.name.starts_with(&filename_input) {
                        cluster = entry.start_cluster;
                    }
                }, vga_index);

                if cluster > 0 {
                    config::PATH_CLUSTER = cluster as u16;
                    config::set_path(&filename_input);
                } else {
                    crate::vga::write::string(vga_index, b"No such directory", crate::vga::buffer::Color::Red);
                    crate::vga::write::newline(vga_index);
                }
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}

fn cmd_chat(args: &[u8], vga_index: &mut isize) {
    crate::vga::screen::clear(vga_index);

    let mut ips = [[0u8; 4]; MAX_IPS];
    let count = parse_ip_args(args, &mut ips);

    if count > 0 {
        app::chat::tcp::handle_conns(vga_index, &ips);
    } else {
        app::chat::tcp::handle_conns(vga_index, &[[0u8; 4]; 4]);
    }
}

fn cmd_clear(_args: &[u8], vga_index: &mut isize) {
    vga::screen::clear(vga_index);
}

fn cmd_dir(_args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            unsafe {
                fs.list_dir(config::PATH_CLUSTER, &[b' '; 11], vga_index);
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

fn cmd_ed(args: &[u8], vga_index: &mut isize) {
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.len() == 0 || filename_input.len() > 12 {
        vga::write::string(vga_index, b"Usage: ed <filename>", vga::buffer::Color::Yellow);
        newline(vga_index);
        return;
    }

    let mut filename = [b' '; 12];
    if let Some(slice) = filename.get_mut(..filename_input.len()) {
        slice.copy_from_slice(filename_input);
    }

    //to_uppercase_ascii(&mut filename);

    vga::screen::clear(vga_index);
    app::editor::edit_file(&filename, vga_index);
    vga::screen::clear(vga_index);
}

fn cmd_ether(args: &[u8], vga_index: &mut isize) {
    app::ether::handle_packet(vga_index);
}

fn cmd_fsck(_args: &[u8], vga_index: &mut isize) {
    run_check(vga_index);
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

fn cmd_snake(_args: &[u8], vga_index: &mut isize) {
    app::snake::menu::menu_loop(vga_index);
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

fn cmd_menu(_args: &[u8], vga_index: &mut isize) {
    // Working sample, but loop without exit
    //app::menu::menu_loop(vga_index);

    let mut label1 = Label { x: 0, y: 0, text: "Play", attr: 0x0F };
    let mut label2 = Label { x: 0, y: 2, text: "Scores", attr: 0x0F };
    let mut label3 = Label { x: 0, y: 4, text: "Quit", attr: 0x0F };

    let mut menu = Container {
        x: 30,
        y: 10,
        children: [&mut label1, &mut label2, &mut label3],
    };

    let mut window = Window {
        x: 20,
        y: 5,
        w: 40,
        h: 15,
        title: Some("Snake Menu"),
        child: Some(&mut menu),
    };

    let mut app = TuiApp::new();
    app.set_root(&mut window);
    app.run();
}

fn cmd_mkdir(args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    if args.len() == 0 || args.len() > 11 {
        crate::vga::write::string(vga_index, b"Usage: mkdir <dirname>", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);
        return;
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut filename: [u8; 11] = [b' '; 11];

            if let Some(slice) = filename.get_mut(..) {
                slice[..args.len()].copy_from_slice(args);
            }

            to_uppercase_ascii(&mut filename);
            unsafe {
                fs.create_subdirectory(&filename, PATH_CLUSTER, vga_index);
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}


fn cmd_mv(args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    if args.len() == 0 {
        crate::vga::write::string(vga_index, b"Usage: mv <old> <new>", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);
        return;
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let (old, new) = split_cmd(args);

            let mut old_filename: [u8; 11] = [b' '; 11];
            let mut new_filename: [u8; 11] = [b' '; 11];

            if new.len() == 0 || old.len() == 0 || old.len() > 11 || new.len() > 11 {
                crate::vga::write::string(vga_index, b"Usage: mv <old> <new>", crate::vga::buffer::Color::Yellow);
                crate::vga::write::newline(vga_index);
                return;
            }

            if let Some(slice) = old_filename.get_mut(..) {
                slice[..old.len()].copy_from_slice(old);
                slice[8..11].copy_from_slice(b"TXT");
            }

            if let Some(slice) = new_filename.get_mut(..) {
                slice[..new.len()].copy_from_slice(new);
                slice[8..11].copy_from_slice(b"TXT");
            }

            to_uppercase_ascii(&mut old_filename);
            to_uppercase_ascii(&mut new_filename);

            unsafe {
                fs.rename_file(PATH_CLUSTER, &old_filename, &new_filename, vga_index);
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}


fn cmd_ping(args: &[u8], vga_index: &mut isize) {
    let mut ips = [[0u8; 4]; MAX_IPS];
    let count = parse_ip_args(args, &mut ips);

    let protocol = 1;
    let identifier = 1342;
    let sequence_no = 1;
    let payload = b"ping from r2"; // optional payload*/

    let mut icmp_buf = [0u8; 256];
    let mut ipv4_buf = [0u8; 1500];

    // Create ICMP packet and encapsulate it in the IPv4 packet.
    let icmp_len = net::icmp::create_packet(8, identifier, sequence_no, payload, &mut icmp_buf);
    let icmp_slice = icmp_buf.get(..icmp_len).unwrap_or(&[]);

    let ipv4_len = net::ipv4::create_packet(ips[0], ips[1], protocol, icmp_slice, &mut ipv4_buf);
    let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);

    vga::write::string(vga_index, b"Sending a ping packet...", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    net::ipv4::send_packet(ipv4_slice);
}

fn cmd_read(args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    if args.len() == 0 || args.len() > 11 {
        crate::vga::write::string(vga_index, b"Usage: read <filename>", crate::vga::buffer::Color::Red);
        crate::vga::write::newline(vga_index);
        return;
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut filename = [b' '; 11];

            filename[..args.len()].copy_from_slice(args);
            filename[8..11].copy_from_slice(b"TXT");

            to_uppercase_ascii(&mut filename);

            unsafe {
                let cluster = fs.list_dir(config::PATH_CLUSTER, &filename, vga_index);

                if cluster > 0 {
                    let mut buf = [0u8; 512];

                    fs.read_file(cluster as u16, &mut buf, vga_index);

                    crate::vga::write::string(vga_index, &buf, crate::vga::buffer::Color::Yellow);
                    crate::vga::write::newline(vga_index);
                } else {
                    crate::vga::write::string(vga_index, b"No such file", crate::vga::buffer::Color::Red);
                    crate::vga::write::newline(vga_index);
                }
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
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
                let ipv4_len = net::ipv4::create_packet(ipv4_header.dest_ip, ipv4_header.source_ip, ipv4_header.protocol, icmp_slice, &mut ipv4_buf);
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

fn cmd_rm(args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    if args.len() == 0 || args.len() > 11 {
        crate::vga::write::string(vga_index, b"Usage: rm <filename>", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);
        return;
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut filename: [u8; 11] = [b' '; 11];

            if let Some(slice) = filename.get_mut(..) {
                slice[..args.len()].copy_from_slice(args);
                slice[8..11].copy_from_slice(b"TXT");
            }

            to_uppercase_ascii(&mut filename);

            unsafe {
                fs.delete_file(PATH_CLUSTER, &filename, vga_index);
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
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

fn cmd_write(args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let (filename, content) = split_cmd(args);

            if filename.len() == 0 || content.len() == 0 {
                vga::write::string(vga_index, b"Usage <filename> <content>", vga::buffer::Color::Yellow);
                vga::write::newline(vga_index);
                return;
            }

            if filename.len() > 8 {
                vga::write::string(vga_index, b"Filename too long (>8)", vga::buffer::Color::Red);
                vga::write::newline(vga_index);
                return;
            }

            let mut name = [b' '; 11];

            if let Some(slice) = name.get_mut(..) {
                slice[..filename.len()].copy_from_slice(filename);
                slice[8..11].copy_from_slice(b"TXT");
            }

            to_uppercase_ascii(&mut name);

            unsafe {
                fs.write_file(PATH_CLUSTER, &name, content, vga_index);
            }
        }
        Err(e) => {
            crate::vga::write::string(vga_index, e.as_bytes(), crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
        }
    }
}

