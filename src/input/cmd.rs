use crate::acpi;
use crate::app;
use crate::audio;
use crate::fs::fat12::block::BlockDevice;
use crate::fs::fat12::entry;
use crate::init::config;
use crate::debug;
use crate::fs::fat12::{block::{Floppy, BlockDevice}, fs::{Fs,Filesystem}, check::run_check};
use crate::init::config::get_path;
use crate::init::config::PATH_CLUSTER;
use crate::init::result;
use crate::input::keyboard::keyboard_loop;
use crate::net;
use crate::time;
use crate::video;
use crate::input::keyboard;
use crate::tui::{widget::{Container, Window, Label}, app::TuiApp};

const KERNEL_VERSION: &[u8] = b"0.8.1";

struct Command {
    name: &'static [u8],
    description: &'static [u8],
    function: fn(args: &[u8]),
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
        name: b"run",
        description: b"loads the binary executable in memory and gives it the control",
        function: cmd_run,
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
    },
    Command {
        name: b"dd",
        description: b"define and dump data; copy and convert across them",
        function: cmd_dd,
        hidden: false,
    }
];

/// Handle takes in an input from keyboard and tries to match it to a defined Command to execute it
/// with given arguments.
pub fn handle(input: &[u8]) {
    let (cmd_name, cmd_args) = split_cmd(input);

    match find_cmd(cmd_name) {
        Some(cmd) => {
            // Call the command function
            (cmd.function)(cmd_args);
        }
        None => {
            if input.is_empty() {
                return;
            }

            // Echo back the input
            error!("Unknown command: ");
            printb!(cmd_name);
            println!();
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
fn cmd_beep(_args: &[u8]) {
    audio::midi::play_melody();
    audio::beep::stop_beep();
}

/// Changes the current directory to one matching an input from keyboard.
fn cmd_cd(args: &[u8]) {
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

    let floppy = Floppy::init();

    // Init the filesystem to look for a match
    match Filesystem::new(&floppy) {
        Ok(fs) => {
            let mut cluster: u16 = 0;

            unsafe {
                fs.for_each_entry(config::PATH_CLUSTER, |entry| {
                    if entry.name.starts_with(&filename_input) {
                        cluster = entry.start_cluster;
                    }
                });

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
fn cmd_chat(args: &[u8]) {
    clear_screen!();

    let mut ips = [[0u8; 4]; MAX_IPS];
    let count = parse_ip_args(args, &mut ips);

    if count > 0 {
        app::chat::tcp::handle_conns(&ips);
    } else {
        // Use dummy IP addresses to 
        app::chat::tcp::handle_conns(&[[0u8; 4]; 4]);
    }
}

/// This just clears the whole screen with black background color.
fn cmd_clear(_args: &[u8]) {
    clear_screen!();
}

/// Dumps the whole debug log to display and tries to write it to the DEBUG.TXT file too if
/// filesystem is reachable.
fn cmd_debug(_args: &[u8]) {
    debug::dump_debug_log_to_file();
}

/// Prints the whole contents of the current directory.
fn cmd_dir(_args: &[u8]) {
    let floppy = Floppy;

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            unsafe {
                fs.for_each_entry(PATH_CLUSTER, | entry | {
                    if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.name[0] == 0xFF {
                        return;
                    }

                    fs.print_name(entry);
                });
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// Echos the arguments back to the display.
fn cmd_echo(args: &[u8]) {
    printb!(args);
    println!();
}

/// Runs a simplistic text editor.
fn cmd_ed(args: &[u8]) {
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
    app::editor::edit_file(&filename);
    clear_screen!();
}

/// Experimental command function to test the Ethernet implementation.
fn cmd_ether(_args: &[u8]) {
    app::ether::handle_packet();
}

/// Filesystem check utility.
fn cmd_fsck(_args: &[u8]) {
    run_check();
}

/// Meta command to dump all non-hidden commands.
fn cmd_help(_args: &[u8]) {
    println!("List of commands:");

    for cmd in COMMANDS {
        if cmd.hidden {
            continue;
        }

        // Print the command name and description
        print!(" ", video::vga::Color::Blue);
        printb!(cmd.name);
        print!(": ", video::vga::Color::White);
        printb!(cmd.description);
        println!();
    }
}

/// Experimental command function to test the HTTP over UDP implementation.
fn cmd_http(_args: &[u8]) {
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
fn cmd_menu(_args: &[u8]) {
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
fn cmd_mkdir(args: &[u8]) {
    if args.len() == 0 || args.len() > 11 {
        warn!("Usage: mkdir <dirname>\n");
        return;
    }

    let floppy = Floppy;

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            let mut filename: [u8; 11] = [b' '; 11];

            if let Some(slice) = filename.get_mut(..) {
                slice[..args.len()].copy_from_slice(args);
            }

            to_uppercase_ascii(&mut filename);
            unsafe {
                fs.create_subdirectory(&filename, PATH_CLUSTER);
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// Renames given <old_name> to <new_name> in the current directory.
fn cmd_mv(args: &[u8]) {
    if args.len() == 0 {
        warn!("Usage: mv <old> <new>");
        return;
    }

    let floppy = Floppy::init();

    match Filesystem::new(&floppy) {
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
                fs.rename_file(PATH_CLUSTER, &old_filename, &new_filename);
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// Sends an ICMP Echo request to the provided IPv4 address.
fn cmd_ping(args: &[u8]) {
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
fn cmd_read(args: &[u8]) {
    if args.len() == 0 || args.len() > 11 {
        warn!("Usage: read <filename>\n");
        return;
    }

    let floppy = Floppy::init();

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            let mut filename = [b' '; 11];

            filename[..args.len()].copy_from_slice(args);
            filename[8..11].copy_from_slice(b"TXT");

            to_uppercase_ascii(&mut filename);

            unsafe {
                // TODO: tix this
                //let cluster = fs.list_dir(config::PATH_CLUSTER, &filename);
                let cluster = 0;

                if cluster > 0 {
                    let mut buf = [0u8; 512];

                    fs.read_file(cluster as u16, &mut buf);

                    print!("Dumping file raw contents:\n", video::vga::Color::DarkYellow);
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
fn cmd_response(_args: &[u8]) {
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
fn cmd_rm(args: &[u8]) {
    if args.len() == 0 || args.len() > 11 {
        warn!("Usage: rm <filename>\n");
        return;
    }

    let floppy = Floppy::init();

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            let mut filename: [u8; 11] = [b' '; 11];

            if let Some(slice) = filename.get_mut(..) {
                slice[..args.len()].copy_from_slice(args);
                slice[8..11].copy_from_slice(b"TXT");
            }

            to_uppercase_ascii(&mut filename);

            unsafe {
                fs.delete_file(PATH_CLUSTER, &filename);
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

fn cmd_run(args: &[u8]) {
    if args.len() == 0 || args.len() > 12 {
        warn!("usage: run <binary name>");
        return;
    }

    // This split_cmd invocation trims the b'\0' tail from the input args.
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.len() == 0 || filename_input.len() > 12 {
        warn!("Usage: run <binary name>\n");
        return;
    }

    // 12 = filename + ext + dot
    let mut filename = [b' '; 12];
    if let Some(slice) = filename.get_mut(..filename_input.len()) {
        slice.copy_from_slice(filename_input);
    }

    let floppy = Floppy::init();

    // Init the filesystem to look for a match
    match Filesystem::new(&floppy) {
        Ok(fs) => {
            unsafe {
                let mut cluster: u16 = 0;
                let mut offset = 0;
                let mut size = 0;

                fs.for_each_entry(config::PATH_CLUSTER, |entry| {
                    if entry.name.starts_with(&filename_input) {
                        cluster = entry.start_cluster;
                        size = entry.file_size;
                        return;
                    }
                });

                rprint!("Size: ");
                rprintn!(size);
                rprint!("\n");

                if cluster == 0 {
                    error!("no such file found");
                    error!();
                    return;
                }

                let load_addr: usize = 0x600_000;

                while size - offset > 0 {
                    let lba = fs.cluster_to_lba(cluster);
                    let mut sector = [0u8; 512];

                    fs.device.read_sector(lba, &mut sector);

                    let dst = load_addr as *mut u8;

                    //core::ptr::copy_nonoverlapping(sector.as_ptr(), dst.add(offset as usize), 512.min((size - offset) as usize));

                    //rprint!("loading binary data to memory segment\n");
                    for i in 0..512 {
                        if let Some(byte) = sector.get(i) {
                            *dst.add(i + offset as usize) = *byte;
                        }
                    }

                    cluster = fs.read_fat12_entry(cluster);

                    rprint!("Cluster: ");
                    rprintn!(cluster);
                    rprint!("\n");

                    if cluster >= 0xFF8 || cluster == 0 {
                        break;
                    }

                    //rprint!("offset++\n");
                    offset += 512;
                }

                //let entry: extern "C" fn(u32) -> u32 = core::mem::transmute((load_addr + 0x00) as *mut u8);
                let entry: extern "C" fn(u32) -> u32 = core::mem::transmute((load_addr + 0x41) as *mut u8);
                let arg: u32 = 5;

                //let result = run_program(entry, arg);
                let result = entry(arg);

                rprint!("Program returned: ");
                rprintn!(result);
                rprint!("\n");

            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

const USER_STACK_SIZE: usize = 0x8000; // 32 KiB
static mut USER_STACK: [u8; USER_STACK_SIZE] = [0; USER_STACK_SIZE];

unsafe extern "C" fn user_program_return() -> ! {
    core::arch::asm!(
        "mov rdi, rax",     
        "jmp {handler}",
        handler = sym handle_program_return,
        options(noreturn)
    );
}

extern "C" fn handle_program_return(retval: u64) {
    rprint!("Program returned: ");
    rprintn!(retval);
    rprint!("\n");
    
    keyboard_loop();
}

unsafe fn run_program(entry: extern "C" fn(u32) -> u32, arg: u32) -> u32 {
    let mut ret: u32;

    // Get stack top
    let user_stack_top = USER_STACK.as_ptr().add(USER_STACK_SIZE);

    let return_addr = user_program_return() as usize;

    core::arch::asm!(
        "mov {old_rsp}, rsp",
        "mov rsp, {stack}",
        "push {ret_addr}",
        "mov rdi, {arg:r}",
        "call {entry}",
        "mov rsp, {old_rsp}",
        stack = in(reg) user_stack_top.offset(-8),
        ret_addr = in(reg) return_addr,
        entry = in(reg) entry,
        old_rsp = lateout(reg) _,
        arg = in(reg) arg,
        out("rax") ret,
        options(nostack),
    );

    ret
}

/// Experimental command function to demonstrate the current state of the shutdown process
/// implemented.
fn cmd_shutdown(_args: &[u8]) {
    print!("\n\n --- Shutting down the system", video::vga::Color::DarkCyan);

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
fn cmd_snake(_args: &[u8]) {
    app::snake::menu::menu_loop();
}

/// Experimental command function to demonstrate the implementation state of the TCP/IP stack.
fn cmd_tcp(_args: &[u8]) {
    app::tcp_handler::handle();
}

/// Prints current time and date in UTC as read from RTC in CMOS.
fn cmd_time(_args: &[u8]) {
    let (y, mo, d, h, m, s) = time::rtc::read_rtc_full();

    print!("RTC Time: ");

    // Hours
    printn!(h as u64);
    print!(":");

    // Minutes
    if m < 10 { 
        print!("0");
    }
    printn!(m as u64);
    print!(":");

    // Seconds
    if s < 10 { 
        print!("0");
    }
    printn!(s as u64);
    println!();

    print!("RTC Date: ");

    // Day of month
    if d < 10 {
        print!("0");
    }
    printn!(d as u64);
    print!("/");

    // Months
    if mo < 10 {
        print!("0");
    }
    printn!(mo as u64);
    print!("/");

    printn!(y as u64);
    println!();
}

/// Experimental command function to show the system uptime.
fn cmd_uptime(_args: &[u8]) {
    let total_seconds = time::acpi::get_uptime_seconds();

    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    let s = total_seconds % 60;

    print!("System uptime: ");

    // Hours
    printn!(h);
    print!(":");

    // Minutes
    if m < 10 {
        print!("0");
    }
    printn!(m);
    print!(":");

    // Seconds
    if s < 10 {
        print!("0");
    }
    printn!(s);
    println!();
}

/// Prints system information set, mainly version and name.
fn cmd_version(_args: &[u8]) {
    print!("Version: ");
    printb!(KERNEL_VERSION);
    println!();
}

/// Experimental command function to demonstrate the possibility of writing to files in FAT12 filesystem.
fn cmd_write(args: &[u8]) {
    let floppy = Floppy::init();

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            let (filename, content) = split_cmd(args);

            if filename.len() == 0 || content.len() == 0 {
                warn!("Usage <filename> <content>\n");
                return;
            }

            if filename.len() > 8 {
                error!("Filename too long (>8)\n");
                return;
            }

            let mut name = [b' '; 11];

            if let Some(slice) = name.get_mut(..) {
                slice[..filename.len()].copy_from_slice(filename);
                slice[8..11].copy_from_slice(b"TXT");
            }

            to_uppercase_ascii(&mut name);

            unsafe {
                fs.write_file(PATH_CLUSTER, &name, content);
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

fn cmd_dd(args: &[u8], vga_index: &mut isize) {
    if args.len() == 0 {
        println!("Usage: dd if=<input> of=<output> [bs=<blocksize>] [count=<blocks>] [skip=<blocks>] [seek=<blocks>] [status=<level>]");
        println!("       dd fs=<filesystem> [bs=<sectorsize>] [if=<input>] [of=<output>] [drive=<device>]");
        println!("Examples:");
        println!("  dd if=file1.txt of=file2.txt bs=256 count=4    - copy with 256-byte blocks");
        println!("  dd if=/dev/zero of=/dev/fda bs=1024 count=100  - write zeros with 1KB blocks");
        println!("  dd if=file1.txt of=file2.txt skip=2 seek=1     - copy with input/output offsets");
        println!("  dd fs=fat12 bs=512                             - format FAT12 with 512-byte sectors");
        println!("  dd fs=raw bs=1024                              - raw format with 1KB block size");
        println!("  dd fs=raw if=data.txt                          - raw format from input file");
        println!("  dd fs=fat12 of=disk.img                        - create FAT12 image file");
        println!("  dd fs=raw if=data.txt of=image.img             - create raw image from file");
        println!("  dd if=file.txt of=copy.txt status=progress     - show transfer progress");
        println!("Block size (bs=) controls sector size for disk operations (1-4096 bytes)");
        return;
    }

    let args_str = core::str::from_utf8(args).unwrap_or("");
    
    let is_format_operation = args_str.starts_with("format") || args_str.contains("fs=");
    
    if is_format_operation {
        let format_args = if args_str.starts_with("format ") {
            &args_str[7..]
        } else if args_str.starts_with("format") {
            &args_str[6..]
        } else {
            args_str
        };
        cmd_dd_format_enhanced(format_args, vga_index);
        return;
    }

    let mut input_file: Option<&str> = None;
    let mut output_file: Option<&str> = None;
    let mut block_size: usize = 512;
    let mut count: Option<usize> = None;
    let mut skip: usize = 0;
    let mut seek: usize = 0;
    let mut status: &str = "none";

    for arg in args_str.split_whitespace() {
        if arg.starts_with("if=") {
            input_file = Some(&arg[3..]);
        } else if arg.starts_with("of=") {
            output_file = Some(&arg[3..]);
        } else if arg.starts_with("bs=") {
            if let Ok(bs) = arg[3..].parse::<usize>() {
                if bs > 0 && bs <= 4096 {
                    block_size = bs;
                } else {
                    error!("Invalid block size (must be 1-4096)\n");
                    return;
                }
            }
        } else if arg.starts_with("count=") {
            if let Ok(c) = arg[6..].parse::<usize>() {
                count = Some(c);
            }
        } else if arg.starts_with("skip=") {
            if let Ok(s) = arg[5..].parse::<usize>() {
                skip = s;
            }
        } else if arg.starts_with("seek=") {
            if let Ok(s) = arg[5..].parse::<usize>() {
                seek = s;
            }
        } else if arg.starts_with("status=") {
            status = &arg[7..];
        }
    }

    match (input_file, output_file) {
        (Some(input), Some(output)) => {
            cmd_dd_copy_enhanced(input, output, block_size, count, skip, seek, status, vga_index);
        }
        _ => {
            error!("Missing if= or of= parameter\n");
            println!("Usage: dd if=<input> of=<output> [bs=<blocksize>] [count=<blocks>] [skip=<blocks>] [seek=<blocks>]");
        }
    }
}

fn cmd_dd_format_enhanced(args: &str, vga_index: &mut isize) {
    let mut filesystem = "fat12";
    let mut drive = "floppy";
    let mut block_size = 512;
    let mut input_file = "";
    let mut output_file = "";
    
    for arg in args.split_whitespace() {
        if arg.starts_with("fs=") {
            filesystem = &arg[3..];
        } else if arg.starts_with("drive=") {
            drive = &arg[6..];
        } else if arg.starts_with("bs=") {
            if let Ok(bs) = arg[3..].parse::<usize>() {
                if bs > 0 && bs <= 4096 {
                    block_size = bs;
                } else {
                    error!("Invalid block size for format (must be 1-4096)\n");
                    return;
                }
            }
        } else if arg.starts_with("if=") {
            input_file = &arg[3..];
        } else if arg.starts_with("of=") {
            output_file = &arg[3..];
        }
    }
    
    match filesystem {
        "fat12" => {
            if output_file.is_empty() {
                cmd_dd_format_fat12_with_block_size(block_size, vga_index);
            } else {
                cmd_dd_format_fat12_to_file(output_file, block_size, vga_index);
            }
        },
        "raw" => {
            if output_file.is_empty() {
                if input_file.is_empty() {
                    cmd_dd_format_raw_with_block_size(block_size, vga_index);
                } else {
                    cmd_dd_format_raw_from_file(input_file, block_size, vga_index);
                }
            } else {
                cmd_dd_format_raw_to_file(input_file, output_file, block_size, vga_index);
            }
        },
        _ => {
            error!("Unsupported filesystem: ");
            printb!(filesystem.as_bytes());
            println!();
            println!("Supported filesystems: fat12, raw");
        }
    }
}

fn cmd_dd_format_fat12(vga_index: &mut isize) {
    cmd_dd_format_fat12_with_block_size(512, vga_index);
}

fn cmd_dd_format_fat12_with_block_size(block_size: usize, vga_index: &mut isize) {
    print!("Formatting floppy disk with FAT12 filesystem (bs=", video::vga::Color::Yellow);
    printn!(block_size as u64);
    println!(")...");
    
    let floppy = Floppy;
    let sectors_per_block = (block_size + 511) / 512;
    
    let mut boot_sector = [0u8; 512];
    
    boot_sector[0] = 0xEB;
    boot_sector[1] = 0x3C;
    boot_sector[2] = 0x90;
    
    boot_sector[3..11].copy_from_slice(b"MSWIN4.1");
    
    boot_sector[11] = (block_size & 0xFF) as u8;
    boot_sector[12] = ((block_size >> 8) & 0xFF) as u8;
    
    boot_sector[13] = sectors_per_block as u8;
    
    boot_sector[14] = 0x01;
    boot_sector[15] = 0x00;
    
    boot_sector[16] = 0x02;
    
    boot_sector[17] = 0xE0;
    boot_sector[18] = 0x00;
    
    boot_sector[19] = 0x40;
    boot_sector[20] = 0x0B;
    
    boot_sector[21] = 0xF0;
    
    boot_sector[22] = 0x09;
    boot_sector[23] = 0x00;
    
    boot_sector[24] = 0x12;
    boot_sector[25] = 0x00;
    
    boot_sector[26] = 0x02;
    boot_sector[27] = 0x00;
    
    boot_sector[28] = 0x00;
    boot_sector[29] = 0x00;
    boot_sector[30] = 0x00;
    boot_sector[31] = 0x00;
    
    boot_sector[54..59].copy_from_slice(b"FAT12");
    
    boot_sector[510] = 0x55;
    boot_sector[511] = 0xAA;
    
    floppy.write_sector(0, &boot_sector, vga_index);
    
    let mut fat_sector = [0u8; 512];
    fat_sector[0] = 0xF0;
    fat_sector[1] = 0xFF;
    fat_sector[2] = 0xFF;
    
    floppy.write_sector(1, &fat_sector, vga_index);
    floppy.write_sector(10, &fat_sector, vga_index);
    
    let zero_sector = [0u8; 512];
    for sector in 2..9 {
        floppy.write_sector(sector, &zero_sector, vga_index);
    }
    for sector in 11..18 {
        floppy.write_sector(sector, &zero_sector, vga_index);
    }
    
    for sector in 19..33 {
        floppy.write_sector(sector, &zero_sector, vga_index);
    }
    
    print!("FAT12 format completed with ", video::vga::Color::Green);
    printn!(block_size as u64);
    println!(" byte sectors");
}

fn cmd_dd_format_raw(vga_index: &mut isize) {
    cmd_dd_format_raw_with_block_size(512, vga_index);
}

fn cmd_dd_format_raw_with_block_size(block_size: usize, vga_index: &mut isize) {
    print!("Performing raw format (zero-fill) with block size ", video::vga::Color::Yellow);
    printn!(block_size as u64);
    println!(" bytes...");
    
    let floppy = Floppy;
    let sectors_per_block = (block_size + 511) / 512;
    let total_blocks = 2880 / sectors_per_block;
    
    let mut zero_buffer = [0u8; 4096];
    if block_size > 4096 {
        error!("Block size too large for raw format (max 4096)\n");
        return;
    }
    
    for block_idx in 0..total_blocks {
        let start_sector = block_idx * sectors_per_block;
        
        for sector_offset in 0..sectors_per_block {
            let sector = start_sector + sector_offset;
            if sector >= 2880 {
                break;
            }
            
            let buffer_start = sector_offset * 512;
            let buffer_end = core::cmp::min(buffer_start + 512, block_size);
            
            if buffer_end > buffer_start {
                let mut sector_buffer = [0u8; 512];
                let copy_len = buffer_end - buffer_start;
                sector_buffer[..copy_len].copy_from_slice(&zero_buffer[buffer_start..buffer_end]);
                floppy.write_sector(sector as u64, &sector_buffer, vga_index);
            }
        }
        
        if block_idx % (total_blocks / 10).max(1) == 0 {
            print!("Progress: ", video::vga::Color::Yellow);
            printn!((block_idx * 100 / total_blocks) as u64);
            println!("%");
        }
    }
    
    print!("Raw format completed - ", video::vga::Color::Green);
    printn!((total_blocks * block_size) as u64);
    println!(" bytes zeroed");
}

fn cmd_dd_copy_enhanced(input: &str, output: &str, block_size: usize, count: Option<usize>, skip: usize, seek: usize, status: &str, vga_index: &mut isize) {
    if input == "/dev/zero" || output.starts_with("/dev/") {
        cmd_dd_raw_device_copy(input, output, block_size, count, skip, seek, status, vga_index);
        return;
    }
    
    let floppy = Floppy;
    
    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut input_filename = [b' '; 11];
            let mut output_filename = [b' '; 11];
            
            if input.len() > 8 || output.len() > 8 {
                error!("Filename too long (max 8 characters)\n");
                return;
            }
            
            input_filename[..input.len()].copy_from_slice(input.as_bytes());
            input_filename[8..11].copy_from_slice(b"TXT");
            to_uppercase_ascii(&mut input_filename);
            
            output_filename[..output.len()].copy_from_slice(output.as_bytes());
            output_filename[8..11].copy_from_slice(b"TXT");
            to_uppercase_ascii(&mut output_filename);
            
            unsafe {
                let input_cluster = fs.list_dir(config::PATH_CLUSTER, &input_filename, vga_index);
                
                if input_cluster > 0 {
                    let mut buffer = [0u8; 4096];
                    let read_size = core::cmp::min(buffer.len(), block_size * 8);
                    let mut file_buffer = [0u8; 512];
                    fs.read_file(input_cluster as u16, &mut file_buffer, vga_index);
                    let copy_size = core::cmp::min(read_size, 512);
                    buffer[..copy_size].copy_from_slice(&file_buffer[..copy_size]);
                    
                    let data_len = buffer[..read_size].iter().position(|&x| x == 0).unwrap_or(read_size);
                    
                    let skip_bytes = skip * block_size;
                    let seek_bytes = seek * block_size;
                    
                    if skip_bytes >= data_len {
                        error!("Skip offset beyond file size\n");
                        return;
                    }
                    
                    let start_pos = skip_bytes;
                    let max_copy_len = data_len - start_pos;
                    
                    let copy_len = if let Some(c) = count {
                        core::cmp::min(c * block_size, max_copy_len)
                    } else {
                        max_copy_len
                    };
                    
                    if copy_len == 0 {
                        println!("No data to copy");
                        return;
                    }
                    
                    let data_slice = &buffer[start_pos..start_pos + copy_len];
                    
                    if status == "progress" || status == "noxfer" {
                        print!("Copying ", video::vga::Color::Yellow);
                        printn!(copy_len as u64);
                        print!(" bytes (bs=", video::vga::Color::Yellow);
                        printn!(block_size as u64);
                        print!(", skip=", video::vga::Color::Yellow);
                        printn!(skip as u64);
                        print!(", seek=", video::vga::Color::Yellow);
                        printn!(seek as u64);
                        println!(")");
                    }
                    
                    fs.write_file(config::PATH_CLUSTER, &output_filename, data_slice, vga_index);
                    
                    print!("Copied ", video::vga::Color::Green);
                    printn!(copy_len as u64);
                    print!(" bytes from ", video::vga::Color::Green);
                    printb!(input.as_bytes());
                    print!(" to ", video::vga::Color::Green);
                    printb!(output.as_bytes());
                    println!();
                    
                    if status == "progress" {
                        let blocks_copied = (copy_len + block_size - 1) / block_size;
                        printn!(blocks_copied as u64);
                        print!(" blocks (", video::vga::Color::Cyan);
                        printn!(block_size as u64);
                        println!(" bytes each) copied");
                    }
                } else {
                    error!("Input file not found: ");
                    printb!(input.as_bytes());
                    println!();
                }
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

fn cmd_dd_raw_device_copy(input: &str, output: &str, block_size: usize, count: Option<usize>, skip: usize, seek: usize, status: &str, vga_index: &mut isize) {
    let floppy = Floppy;
    
    if status == "progress" || status == "noxfer" {
        print!("Raw device copy: ", video::vga::Color::Yellow);
        printb!(input.as_bytes());
        print!(" -> ", video::vga::Color::Yellow);
        printb!(output.as_bytes());
        print!(" (bs=", video::vga::Color::Yellow);
        printn!(block_size as u64);
        println!(")");
    }
    
    let sectors_per_block = (block_size + 511) / 512;
    let start_sector = skip * sectors_per_block;
    let output_start_sector = seek * sectors_per_block;
    
    let total_blocks = if let Some(c) = count { c } else { 2880 / sectors_per_block };
    
    if input == "/dev/zero" {
        let mut zero_buffer = [0u8; 4096];
        if block_size > 4096 {
            error!("Block size too large for device copy (max 4096)\n");
            return;
        }
        
        for block_idx in 0..total_blocks {
            let sector = output_start_sector + (block_idx * sectors_per_block);
            
            if sector + sectors_per_block > 2880 {
                break;
            }
            
            for sector_offset in 0..sectors_per_block {
                let current_sector = sector + sector_offset;
                let buffer_start = sector_offset * 512;
                let buffer_end = core::cmp::min(buffer_start + 512, block_size);
                
                if buffer_end > buffer_start {
                    let mut sector_buffer = [0u8; 512];
                    let copy_len = buffer_end - buffer_start;
                    sector_buffer[..copy_len].copy_from_slice(&zero_buffer[buffer_start..buffer_end]);
                    floppy.write_sector(current_sector as u64, &sector_buffer, vga_index);
                }
            }
            
            if status == "progress" && block_idx % 10 == 0 {
                print!("Progress: ", video::vga::Color::Yellow);
                printn!((block_idx * 100 / total_blocks) as u64);
                println!("%");
            }
        }
        
        print!("Wrote ", video::vga::Color::Green);
        printn!((total_blocks * block_size) as u64);
        println!(" zero bytes to device");
    } else if output.starts_with("/dev/") {
        let mut buffer = [0u8; 4096];
        if block_size > 4096 {
            error!("Block size too large for device copy (max 4096)\n");
            return;
        }
        
        for block_idx in 0..total_blocks {
            let input_sector = start_sector + (block_idx * sectors_per_block);
            let output_sector = output_start_sector + (block_idx * sectors_per_block);
            
            if input_sector + sectors_per_block > 2880 || output_sector + sectors_per_block > 2880 {
                break;
            }
            
            for sector_offset in 0..sectors_per_block {
                let current_input_sector = input_sector + sector_offset;
                let current_output_sector = output_sector + sector_offset;
                let buffer_start = sector_offset * 512;
                let buffer_end = core::cmp::min(buffer_start + 512, block_size);
                
                if buffer_end > buffer_start {
                    let mut sector_buffer = [0u8; 512];
                    floppy.read_sector(current_input_sector as u64, &mut sector_buffer, vga_index);
                    
                    let copy_len = buffer_end - buffer_start;
                    buffer[buffer_start..buffer_end].copy_from_slice(&sector_buffer[..copy_len]);
                    
                    floppy.write_sector(current_output_sector as u64, &sector_buffer, vga_index);
                }
            }
            
            if status == "progress" && block_idx % 10 == 0 {
                print!("Progress: ", video::vga::Color::Yellow);
                printn!((block_idx * 100 / total_blocks) as u64);
                println!("%");
            }
        }
        
        print!("Copied ", video::vga::Color::Green);
        printn!((total_blocks * block_size) as u64);
        println!(" bytes between devices");
    }
}

fn cmd_dd_format_raw_from_file(input_file: &str, block_size: usize, vga_index: &mut isize) {
    print!("Performing raw format from file ", video::vga::Color::Yellow);
    printb!(input_file.as_bytes());
    print!(" with block size ", video::vga::Color::Yellow);
    printn!(block_size as u64);
    println!(" bytes...");
    
    let floppy = Floppy;
    
    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut input_filename = [b' '; 11];
            
            if input_file.len() > 8 {
                error!("Input filename too long (max 8 characters)\n");
                return;
            }
            
            input_filename[..input_file.len()].copy_from_slice(input_file.as_bytes());
            input_filename[8..11].copy_from_slice(b"TXT");
            to_uppercase_ascii(&mut input_filename);
            
            unsafe {
                let input_cluster = fs.list_dir(config::PATH_CLUSTER, &input_filename, vga_index);
                
                if input_cluster > 0 {
                    let mut buffer = [0u8; 4096];
                    let read_size = core::cmp::min(buffer.len(), block_size * 8);
                    let mut file_buffer = [0u8; 512];
                    fs.read_file(input_cluster as u16, &mut file_buffer, vga_index);
                    let copy_size = core::cmp::min(read_size, 512);
                    buffer[..copy_size].copy_from_slice(&file_buffer[..copy_size]);
                    
                    let data_len = buffer[..read_size].iter().position(|&x| x == 0).unwrap_or(read_size);
                    
                    let sectors_per_block = (block_size + 511) / 512;
                    let total_blocks = 2880 / sectors_per_block;
                    
                    for block_idx in 0..total_blocks {
                        let start_sector = block_idx * sectors_per_block;
                        
                        for sector_offset in 0..sectors_per_block {
                            let sector = start_sector + sector_offset;
                            if sector >= 2880 {
                                break;
                            }
                            
                            let buffer_start = sector_offset * 512;
                            let buffer_end = core::cmp::min(buffer_start + 512, block_size);
                            
                            if buffer_end > buffer_start {
                                let mut sector_buffer = [0u8; 512];
                                let copy_len = buffer_end - buffer_start;
                                
                                let data_offset = (block_idx * block_size + buffer_start) % data_len;
                                let available_data = core::cmp::min(copy_len, data_len - data_offset);
                                
                                if available_data > 0 {
                                    sector_buffer[..available_data].copy_from_slice(&buffer[data_offset..data_offset + available_data]);
                                }
                                
                                floppy.write_sector(sector as u64, &sector_buffer, vga_index);
                            }
                        }
                        
                        if block_idx % (total_blocks / 10).max(1) == 0 {
                            print!("Progress: ", video::vga::Color::Yellow);
                            printn!((block_idx * 100 / total_blocks) as u64);
                            println!("%");
                        }
                    }
                    
                    print!("Raw format from file completed - ", video::vga::Color::Green);
                    printn!((total_blocks * block_size) as u64);
                    println!(" bytes written");
                } else {
                    error!("Input file not found: ");
                    printb!(input_file.as_bytes());
                    println!();
                }
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

fn cmd_dd_format_fat12_to_file(output_file: &str, block_size: usize, vga_index: &mut isize) {
    print!("Formatting FAT12 to file ", video::vga::Color::Yellow);
    printb!(output_file.as_bytes());
    print!(" with block size ", video::vga::Color::Yellow);
    printn!(block_size as u64);
    println!(" bytes...");
    
    let floppy = Floppy;
    
    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut output_filename = [b' '; 11];
            
            if output_file.len() > 8 {
                error!("Output filename too long (max 8 characters)\n");
                return;
            }
            
            output_filename[..output_file.len()].copy_from_slice(output_file.as_bytes());
            output_filename[8..11].copy_from_slice(b"IMG");
            to_uppercase_ascii(&mut output_filename);
            
            let mut image_data = [0u8; 1474560];
            
            let sectors_per_block = (block_size + 511) / 512;
            
            let mut boot_sector = [0u8; 512];
            
            boot_sector[0] = 0xEB;
            boot_sector[1] = 0x3C;
            boot_sector[2] = 0x90;
            
            boot_sector[3..11].copy_from_slice(b"MSWIN4.1");
            
            boot_sector[11] = (block_size & 0xFF) as u8;
            boot_sector[12] = ((block_size >> 8) & 0xFF) as u8;
            
            boot_sector[13] = sectors_per_block as u8;
            
            boot_sector[14] = 0x01;
            boot_sector[15] = 0x00;
            
            boot_sector[16] = 0x02;
            
            boot_sector[17] = 0xE0;
            boot_sector[18] = 0x00;
            
            boot_sector[19] = 0x40;
            boot_sector[20] = 0x0B;
            
            boot_sector[21] = 0xF0;
            
            boot_sector[22] = 0x09;
            boot_sector[23] = 0x00;
            
            boot_sector[24] = 0x12;
            boot_sector[25] = 0x00;
            
            boot_sector[26] = 0x02;
            boot_sector[27] = 0x00;
            
            boot_sector[28] = 0x00;
            boot_sector[29] = 0x00;
            boot_sector[30] = 0x00;
            boot_sector[31] = 0x00;
            
            boot_sector[54..59].copy_from_slice(b"FAT12");
            
            boot_sector[510] = 0x55;
            boot_sector[511] = 0xAA;
            
            image_data[..512].copy_from_slice(&boot_sector);
            
            let mut fat_sector = [0u8; 512];
            fat_sector[0] = 0xF0;
            fat_sector[1] = 0xFF;
            fat_sector[2] = 0xFF;
            
            image_data[512..1024].copy_from_slice(&fat_sector);
            image_data[5120..5632].copy_from_slice(&fat_sector);
            
            unsafe {
                fs.write_file(config::PATH_CLUSTER, &output_filename, &image_data, vga_index);
            }
            
            print!("FAT12 image created: ", video::vga::Color::Green);
            printb!(output_file.as_bytes());
            println!();
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

fn cmd_dd_format_raw_to_file(input_file: &str, output_file: &str, block_size: usize, vga_index: &mut isize) {
    print!("Creating raw image ", video::vga::Color::Yellow);
    printb!(output_file.as_bytes());
    
    if !input_file.is_empty() {
        print!(" from ", video::vga::Color::Yellow);
        printb!(input_file.as_bytes());
    }
    
    print!(" with block size ", video::vga::Color::Yellow);
    printn!(block_size as u64);
    println!(" bytes...");
    
    let floppy = Floppy;
    
    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            let mut output_filename = [b' '; 11];
            
            if output_file.len() > 8 {
                error!("Output filename too long (max 8 characters)\n");
                return;
            }
            
            output_filename[..output_file.len()].copy_from_slice(output_file.as_bytes());
            output_filename[8..11].copy_from_slice(b"IMG");
            to_uppercase_ascii(&mut output_filename);
            
            let mut image_data = [0u8; 1474560];
            
            if !input_file.is_empty() {
                let mut input_filename = [b' '; 11];
                
                if input_file.len() > 8 {
                    error!("Input filename too long (max 8 characters)\n");
                    return;
                }
                
                input_filename[..input_file.len()].copy_from_slice(input_file.as_bytes());
                input_filename[8..11].copy_from_slice(b"TXT");
                to_uppercase_ascii(&mut input_filename);
                
                unsafe {
                    let input_cluster = fs.list_dir(config::PATH_CLUSTER, &input_filename, vga_index);
                    
                    if input_cluster > 0 {
                        let mut buffer = [0u8; 4096];
                        let mut file_buffer = [0u8; 512];
                        fs.read_file(input_cluster as u16, &mut file_buffer, vga_index);
                        buffer[..512].copy_from_slice(&file_buffer);
                        
                        let data_len = buffer.iter().position(|&x| x == 0).unwrap_or(4096);
                        
                        for i in 0..image_data.len() {
                            if data_len > 0 {
                                image_data[i] = buffer[i % data_len];
                            }
                        }
                    } else {
                        error!("Input file not found: ");
                        printb!(input_file.as_bytes());
                        println!();
                        return;
                    }
                }
            }
            
            unsafe {
                fs.write_file(config::PATH_CLUSTER, &output_filename, &image_data, vga_index);
            }
            
            print!("Raw image created: ", video::vga::Color::Green);
            printb!(output_file.as_bytes());
            print!(" (", video::vga::Color::Green);
            printn!(image_data.len() as u64);
            println!(" bytes)");
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

