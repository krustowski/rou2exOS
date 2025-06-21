use crate::acpi;
use crate::app;
use crate::audio;
use crate::init::config;
use crate::debug;
use crate::fs::fat12::{block::Floppy, fs::Fs, check::run_check};
use crate::init::config::PATH_CLUSTER;
use crate::net;
use crate::time;
use crate::vga;
use crate::vga::write::newline;
use crate::input::keyboard;
use crate::tui::{widget::{Container, Window, Label}, app::TuiApp};

const KERNEL_VERSION: &[u8] = b"0.7.6";

struct Command {
    name: &'static [u8],
    description: &'static [u8],
    function: fn(args: &[u8], vga_index: &mut isize),
    hidden: bool,
}

static COMMANDS: &[Command] = &[
    Command {
        name: b"beep",
        description: b"beeps",
        function: cmd_beep,
        hidden: false,
    },
    Command {
        name: b"cd",
        description: b"changes the current directory",
        function: cmd_cd,
        hidden: false,
    },
    Command {
        name: b"chat",
        description: b"starts a chat",
        function: cmd_chat,
        hidden: false,
    },
    Command {
        name: b"cls",
        description: b"clears the screen",
        function: cmd_clear,
        hidden: false,
    },
    Command {
        name: b"debug",
        description: b"dumps the debug log into a file",
        function: cmd_debug,
        hidden: false,
    },
    Command {
        name: b"dir",
        description: b"lists the current directory",
        function: cmd_dir,
        hidden: false,
    },
    Command {
        name: b"echo",
        description: b"echos the arguments",
        function: cmd_echo,
        hidden: false,
    },
    Command {
        name: b"ed",
        description: b"runs a minimalistic text editor",
        function: cmd_ed,
        hidden: false,
    },
    Command {
        name: b"ether",
        description: b"runs the Ethernet frame handler",
        function: cmd_ether,
        hidden: true,
    },
    Command {
        name: b"fsck",
        description: b"runs the filesystem check",
        function: cmd_fsck,
        hidden: false,
    },
    Command {
        name: b"help",
        description: b"shows this output",
        function: cmd_help,
        hidden: false,
    },
    Command {
        name: b"http",
        description: b"runs a simple HTTP/UDP handler",
        function: cmd_http,
        hidden: true,
    },
    Command {
        name: b"menu",
        description: b"renders a sample menu",
        function: cmd_menu,
        hidden: true,
    },
    Command {
        name: b"mkdir",
        description: b"creates a subdirectory",
        function: cmd_mkdir,
        hidden: false,
    },
    Command {
        name: b"mv",
        description: b"renames a file",
        function: cmd_mv,
        hidden: false,
    },
    Command {
        name: b"ping",
        description: b"pings the host over the serial line (ICMP/SLIP)",
        function: cmd_ping,
        hidden: true,
    },
    Command {
        name: b"read",
        description: b"prints the output of a file",
        function: cmd_read,
        hidden: false,
    },
    Command {
        name: b"response",
        description: b"waits for ICMP/SLIP request to come, then sends a response back",
        function: cmd_response,
        hidden: false,
    },
    Command {
        name: b"rm",
        description: b"removes a file",
        function: cmd_rm,
        hidden: false,
    },
    Command {
        name: b"shutdown",
        description: b"shuts down the system",
        function: cmd_shutdown,
        hidden: false,
    },
    Command {
        name: b"snake",
        description: b"runs a simple VGA text mode snake-like game",
        function: cmd_snake,
        hidden: false,
    },
    Command {
        name: b"tcp",
        description: b"tests the TCP implementation",
        function: cmd_tcp,
        hidden: true,
    },
    Command {
        name: b"time",
        description: b"prints system time and date",
        function: cmd_time,
        hidden: false,
    },
    Command {
        name: b"uptime",
        description: b"prints system uptime",
        function: cmd_uptime,
        hidden: true,
    },
    Command {
        name: b"version",
        description: b"prints the kernel version",
        function: cmd_version,
        hidden: false,
    },
    Command {
        name: b"write",
        description: b"writes arguments to a sample file on floppy",
        function: cmd_write,
        hidden: false,
    }
];

/// Handle takes in an input from keyboard and tries to match it to a defined Command to execute it
/// with given arguments.
pub fn handle(input: &[u8], vga_index: &mut isize) {
    let (cmd_name, cmd_args) = split_cmd(input);

    match find_cmd(cmd_name) {
        Some(cmd) => {
            // Call the command function
            (cmd.function)(cmd_args, vga_index);
        }
        None => {
            if input.is_empty() {
                return;
            }

            // Echo back the input
            error!("Unknown command: ");
            printb!(cmd_name);
        }
    }
}

//
//  HELPER FUNCTIONS
//

#[allow(clippy::manual_find)]
/// Loops over the slice of defined commands and returns an Option of matching command via its
/// name, or None otherwise.
fn find_cmd(name: &[u8]) -> Option<&'static Command> {
    for cmd in COMMANDS {
        if cmd.name == name {
            return Some(cmd);
        }
    }
    None
}

/// Splits the provided `input` in to tokens, where the delimitor is a single whitespace (space).
pub fn split_cmd(input: &[u8]) -> (&[u8], &[u8]) {
    // Find the first space
    if let Some(pos) = input.iter().position(|&c| c == b' ') {
        let (cmd, args) = input.split_at(pos);
        // Skip the space character for args
        let args_slice = args.get(1..).unwrap_or(&[]);
        (cmd, args_slice)
    } else {
        // No space found, entire input is the command
        (input, &[])
    }
}

/// Defines the maximum amount of IPv4 addresses that could be parsed from an input.
const MAX_IPS: usize = 4;

/// This function takes in an input (&[u8]) of various length, and parses it into IPv4 addresses
/// (up to MAX_IPS). Returns the parsed count of addresses.
fn parse_ip_args(input: &[u8], out: &mut [[u8; 4]; MAX_IPS]) -> usize {
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

/// Used to make the FAT12-formatted filename into UPPERCASE.
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

/// Used to test the sound module, plays the mystery melody.
fn cmd_beep(_args: &[u8], _vga_index: &mut isize) {
    audio::midi::play_melody();
    audio::beep::stop_beep();
}

/// Changes the current directory to one matching an input from keyboard.
fn cmd_cd(args: &[u8], vga_index: &mut isize) {
    // 12 = name + extension + dot
    if args.len() == 0 || args.len() > 12 {
        unsafe {
            config::PATH_CLUSTER = 0;
            config::set_path(b"/");
        }
        return;
    }

    // This split_cmd invocation trims the b'\0' tail from the input args.
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.len() == 0 || filename_input.len() > 12 {
        warn!("Usage: cd <dirname>\n");
        return;
    }

    // 12 = filename + ext + dot
    let mut filename = [b' '; 12];
    if let Some(slice) = filename.get_mut(..filename_input.len()) {
        slice.copy_from_slice(filename_input);
    }

    let floppy = Floppy;

    // Init the filesystem to look for a match
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
                    error!("No such directory\n");
                }
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// Clears the screen and starts the TCP server accepting connections on TCP/12345. 
fn cmd_chat(args: &[u8], vga_index: &mut isize) {
    crate::vga::screen::clear(vga_index);

    let mut ips = [[0u8; 4]; MAX_IPS];
    let count = parse_ip_args(args, &mut ips);

    if count > 0 {
        app::chat::tcp::handle_conns(vga_index, &ips);
    } else {
        // Use dummy IP addresses to 
        app::chat::tcp::handle_conns(vga_index, &[[0u8; 4]; 4]);
    }
}

/// This just clears the whole screen with black background color.
fn cmd_clear(_args: &[u8], vga_index: &mut isize) {
    clear_screen!();
}

/// Dumps the whole debug log to display and tries to write it to the DEBUG.TXT file too if
/// filesystem is reachable.
fn cmd_debug(_args: &[u8], vga_index: &mut isize) {
    debug::dump_debug_log_to_file(vga_index);
}

/// Prints the whole contents of the current directory.
fn cmd_dir(_args: &[u8], vga_index: &mut isize) {
    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            unsafe {
                fs.list_dir(config::PATH_CLUSTER, &[b' '; 11], vga_index);
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// Echos the arguments back to the display.
fn cmd_echo(args: &[u8], _vga_index: &mut isize) {
    printb!(args);
    println!();
}

/// Runs a simplistic text editor.
fn cmd_ed(args: &[u8], vga_index: &mut isize) {
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.len() == 0 || filename_input.len() > 12 {
        warn!("Usage: ed <filename>\n");
        return;
    }

    // Copy the input into a space-padded slice
    let mut filename = [b' '; 12];
    if let Some(slice) = filename.get_mut(..filename_input.len()) {
        slice.copy_from_slice(filename_input);
    }

    //to_uppercase_ascii(&mut filename);

    // Run the editor
    app::editor::edit_file(&filename, vga_index);
    clear_screen!();
}

/// Experimental command function to test the Ethernet implementation.
fn cmd_ether(_args: &[u8], vga_index: &mut isize) {
    app::ether::handle_packet(vga_index);
}

/// Filesystem check utility.
fn cmd_fsck(_args: &[u8], vga_index: &mut isize) {
    run_check(vga_index);
}

/// Meta command to dump all non-hidden commands.
fn cmd_help(_args: &[u8], _vga_index: &mut isize) {
    println!("List of commands:");

    for cmd in COMMANDS {
        if cmd.hidden {
            continue;
        }

        // Print the command name and description
        print!(" ", vga::writer::Color::Blue);
        printb!(cmd.name);
        print!(": ", vga::writer::Color::White);
        printb!(cmd.description);
        println!();
    }
}

/// Experimental command function to test the HTTP over UDP implementation.
fn cmd_http(_args: &[u8], _vga_index: &mut isize) {
    fn callback(packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = net::ipv4::parse_packet(packet) {
            // Match only UDP
            if ipv4_header.protocol != 17 {
                return 1;
            }

            // Handle the connection
            return app::http_udp::udp_handler(&ipv4_header, ipv4_payload);
        }
        0
    }

    println!("Starting a simple HTTP/UDP handler (hit any key to interrupt)...");

    loop {
        // Run the receive loop = try to extract an encapsulated IPv4 packet in SLIP
        let ret = net::ipv4::receive_loop(callback);

        if ret == 0 {
            println!("Received a HTTP request, sending response");
        } else if ret == 3 {
            println!("Keyboard interrupt");
            break;
        }
    }
}

/// Experimental command function to evaluate the current TUI rendering options.
fn cmd_menu(_args: &[u8], _vga_index: &mut isize) {
    // Working sample, but loops without exit
    //app::menu::menu_loop(vga_index);

    // Set the labels
    let mut label1 = Label { x: 0, y: 0, text: "Play", attr: 0x0F };
    let mut label2 = Label { x: 0, y: 2, text: "Scores", attr: 0x0F };
    let mut label3 = Label { x: 0, y: 4, text: "Quit", attr: 0x0F };

    // Create a container to hold all labels
    let mut menu = Container {
        x: 30,
        y: 10,
        children: [&mut label1, &mut label2, &mut label3],
    };

    // Set the dimensions of a TUI window to render it with a proper title in the middle top
    let mut window = Window {
        x: 20,
        y: 5,
        w: 40,
        h: 15,
        title: Some("Snake Menu"),
        child: Some(&mut menu),
    };

    // Run the experimental construction
    let mut app = TuiApp::new();
    app.set_root(&mut window);
    app.run();
}

/// Creates new subdirectory in the current directory.
fn cmd_mkdir(args: &[u8], vga_index: &mut isize) {
    if args.len() == 0 || args.len() > 11 {
        warn!("Usage: mkdir <dirname>\n");
        return;
    }

    let floppy = Floppy;

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
            error!(e);
            error!();
        }
    }
}

/// Renames given <old_name> to <new_name> in the current directory.
fn cmd_mv(args: &[u8], vga_index: &mut isize) {
    if args.len() == 0 {
        warn!("Usage: mv <old> <new>");
        return;
    }

    let floppy = Floppy;

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let (old, new) = split_cmd(args);

            let mut old_filename: [u8; 11] = [b' '; 11];
            let mut new_filename: [u8; 11] = [b' '; 11];

            if new.len() == 0 || old.len() == 0 || old.len() > 11 || new.len() > 11 {
                warn!("Usage: mv <old> <new>");
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
            error!(e);
            error!();
        }
    }
}

/// Sends an ICMP Echo request to the provided IPv4 address.
fn cmd_ping(args: &[u8], _vga_index: &mut isize) {
    // Extract the address(es) from the input
    let mut ips = [[0u8; 4]; MAX_IPS];
    let _count = parse_ip_args(args, &mut ips);

    // Set the ICMP parameters
    let protocol = 1;
    let identifier = 1342;
    let sequence_no = 1;
    let payload = b"iEcho request from r2";

    // Buffers for ICMP and IPv4 packets (ICMP packet prefixed by an IPv4 header)
    let mut icmp_buf = [0u8; 256];
    let mut ipv4_buf = [0u8; 1500];

    // Create ICMP packet and encapsulate it in the IPv4 packet
    let icmp_len = net::icmp::create_packet(8, identifier, sequence_no, payload, &mut icmp_buf);
    let icmp_slice = icmp_buf.get(..icmp_len).unwrap_or(&[]);

    // Use the prepared ICMP packet as payload for IPv4 packet
    let ipv4_len = net::ipv4::create_packet(ips[0], ips[1], protocol, icmp_slice, &mut ipv4_buf);
    let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);

    println!("Sending ICMP Echo request...");

    net::ipv4::send_packet(ipv4_slice);
}

/// This command function takes the argument, then tries to find a matching filename in the current
/// directory, and finally it dumps its content to screen.
fn cmd_read(args: &[u8], vga_index: &mut isize) {
    if args.len() == 0 || args.len() > 11 {
        warn!("Usage: read <filename>\n");
        return;
    }

    let floppy = Floppy;

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

                    print!("Dumping file raw contents:\n", vga::writer::Color::DarkYellow);
                    printb!(&buf);
                    println!();
                } else {
                    error!("No such file found");
                }
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// An experimental demonstration of the ICMP Echo request handler. The implementation sends ICMP
/// Echo response back to the original sender via IPv4/SLIP.
fn cmd_response(_args: &[u8], vga_index: &mut isize) {
    fn callback(packet: &[u8]) -> u8 {
        if let Some((ipv4_header, ipv4_payload)) = net::ipv4::parse_packet(packet) {
            // Match only ICMP packets
            if ipv4_header.protocol != 1 {
                return 1;
            }

            // Extract the ICMP header and (optional) payload
            if let Some((icmp_header, icmp_payload)) = net::icmp::parse_packet(ipv4_payload) {
                // Type 8 is Echo request
                if icmp_header.icmp_type != 8 {
                    return 2;
                }

                // Prepare buffers for new packets.
                let mut icmp_buf = [0u8; 64];
                let mut ipv4_buf = [0u8; 1500];

                // Create an ICMP Echo response packet...
                let icmp_len = net::icmp::create_packet(0, icmp_header.identifier, icmp_header.sequence_number, icmp_payload, &mut icmp_buf);
                let icmp_slice = icmp_buf.get(..icmp_len).unwrap_or(&[]);

                // ...and prefix it with IPv4 header.
                let ipv4_len = net::ipv4::create_packet(ipv4_header.dest_ip, ipv4_header.source_ip, ipv4_header.protocol, icmp_slice, &mut ipv4_buf);
                let ipv4_slice = ipv4_buf.get(..ipv4_len).unwrap_or(&[]);

                net::ipv4::send_packet(ipv4_slice);
            }
        }
        0
    }

    println!("Waiting for an ICMP echo request (hit any key to interrupt)...");

    loop {
        // Start the receive loop where SLIP frames are extracted from serial line and passed into
        // the callback when complete
        let ret = net::ipv4::receive_loop(callback);

        match ret {
            0 => {
                println!("Received ICMP Echo request, sending Echo response back");
            }
            2 => {
                println!("Received ICMP packet (not the Echo request), ignoring");
            }
            3 => {
                println!("Keyboard interrupt");
                break;
            }
            _ => {
                // Hide this as it would spam the screen 
                //println!("Unknown IPv4 protocol number (not ICMP)");
            }
        }
    }
}

/// Removes a file in the current directory according to the input.
fn cmd_rm(args: &[u8], vga_index: &mut isize) {
    if args.len() == 0 || args.len() > 11 {
        warn!("Usage: rm <filename>\n");
        return;
    }

    let floppy = Floppy;

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
            error!(e);
            error!();
        }
    }
}

/// Experimental command function to demonstrate the current state of the shutdown process
/// implemented.
fn cmd_shutdown(_args: &[u8], _vga_index: &mut isize) {
    print!("\n\n --- Shutting down the system", vga::writer::Color::DarkCyan);

    // Burn some CPU time
    for _ in 0..3 {
        for _ in 0..3_500_000 {
            unsafe {
                core::arch::asm!("nop");
            }
        }
        printb!(b". ");
    }

    // Invoke the ACPI shutdown attempt (if present)
    acpi::shutdown::shutdown();
}

/// Meta command to run the Snake game.
fn cmd_snake(_args: &[u8], vga_index: &mut isize) {
    app::snake::menu::menu_loop(vga_index);
}

/// Experimental command function to demonstrate the implementation state of the TCP/IP stack.
fn cmd_tcp(_args: &[u8], vga_index: &mut isize) {
    app::tcp_handler::handle(vga_index);
}

/// Prints current time and date in UTC.
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

