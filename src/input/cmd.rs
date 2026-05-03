use crate::acpi;
use crate::audio;
use crate::debug;
use crate::fs::fat12::{block::Floppy, check::run_check, fs::{fat83, Filesystem}};
use crate::fs::iso9660::Iso9660;
use crate::fs::vfs;
use crate::init::config;
use crate::input::keyboard;
use crate::time;
use crate::video::vga::Color;

const KERNEL_VERSION: &[u8] = b"0.11.0";

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
        name: b"bg",
        description: b"runs an ELF binary in background",
        function: cmd_bg,
        hidden: false,
    },
    Command {
        name: b"cd",
        description: b"changes the current directory",
        function: cmd_cd,
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
        hidden: true,
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
        name: b"fg",
        description: b"runs an ELF binary in foreground",
        function: cmd_fg,
        hidden: false,
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
        name: b"hlt",
        description: b"shuts down the system",
        function: cmd_hlt,
        hidden: false,
    },
    Command {
        name: b"kill",
        description: b"makes a process dead",
        function: cmd_kill,
        hidden: false,
    },
    /*Command {
        name: b"menu",
        description: b"renders a sample menu",
        function: cmd_menu,
        hidden: true,
    },*/
    Command {
        name: b"mkdir",
        description: b"creates a subdirectory",
        function: cmd_mkdir,
        hidden: false,
    },
    Command {
        name: b"mount",
        description: b"lists the VFS mount table",
        function: cmd_mount,
        hidden: false,
    },
    Command {
        name: b"mv",
        description: b"renames a file",
        function: cmd_mv,
        hidden: false,
    },
    Command {
        name: b"read",
        description: b"prints the output of a file",
        function: cmd_read,
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
        hidden: true,
    },
    Command {
        name: b"time",
        description: b"prints system time and date",
        function: cmd_time,
        hidden: false,
    },
    Command {
        name: b"ts",
        description: b"lists currently running tasks",
        function: cmd_ts,
        hidden: false,
    },
    Command {
        name: b"uptime",
        description: b"prints system uptime",
        function: cmd_uptime,
        hidden: true,
    },
    Command {
        name: b"ver",
        description: b"prints the kernel version",
        function: cmd_ver,
        hidden: false,
    },
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

fn parse_u64(bytes: &[u8]) -> Option<u64> {
    let mut value: u64 = 0;

    for &b in bytes {
        if b < b'0' || b > b'9' {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((b - b'0') as u64)?;
    }

    Some(value)
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

/// Runs an ELF binary in background (won't make kernel shell Idle).
fn cmd_bg(args: &[u8]) {
    if args.is_empty() {
        warn!("usage: bg <binary name>\n");
        return;
    }

    // This split_cmd invocation trims the b'\0' tail from the input args.
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.is_empty() || filename_input.len() > 8 {
        warn!("Usage: bg <binary name>\n");
        return;
    }

    super::elf::run_elf(filename_input, args, super::elf::RunMode::Background);
}

fn cmd_fg(args: &[u8]) {
    if args.is_empty() {
        warn!("usage: fg <binary name>\n");
        return;
    }

    // This split_cmd invocation trims the b'\0' tail from the input args.
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.is_empty() || filename_input.len() > 8 {
        warn!("Usage: fg <binary name>\n");
        return;
    }

    super::elf::run_elf(filename_input, args, super::elf::RunMode::Foreground);
}

fn cmd_cd(args: &[u8]) {
    let (name_input, _) = keyboard::split_cmd(args);
    if name_input.is_empty() { return; }

    // Reset to VFS root.
    if name_input == b"/" {
        if let Some(mut c) = config::SYSTEM_CONFIG.try_lock() { c.set_path(b"/", 0); }
        return;
    }

    let floppy = Floppy::init();
    let fs = match Filesystem::new(&floppy) {
        Ok(f) => f,
        Err(e) => { error!(e); error!(); return; }
    };

    // Snapshot current state without holding the lock across I/O.
    let (cur_path_buf, cur_len, cwd) = {
        let c = match config::SYSTEM_CONFIG.try_lock() {
            Some(c) => c,
            None => { error!("cd: config lock unavailable\n"); return; }
        };
        let mut buf = [0u8; 32];
        let p = c.get_path();
        buf[..p.len()].copy_from_slice(p);
        (buf, p.len(), c.get_path_cluster())
    };
    let cur_path = &cur_path_buf[..cur_len];

    // ISO9660 absolute path handling (must be checked before FAT12 `cd ..`).
    if name_input.starts_with(b"/") {
        if let Some(iso_rel) = vfs::try_iso9660_absolute(name_input) {
            match Iso9660::probe() {
                None => { error!("iso: device not available\n"); return; }
                Some(iso) => {
                    let ok = if iso_rel.is_empty() {
                        true // mount root always exists
                    } else {
                        match iso.resolve(iso_rel) {
                            Some(e) => e.is_dir,
                            None => false,
                        }
                    };
                    if !ok { error!("no such directory\n"); return; }
                    let n = name_input.len().min(32);
                    if let Some(mut c) = config::SYSTEM_CONFIG.try_lock() {
                        c.set_path(&name_input[..n], 0);
                    }
                    return;
                }
            }
        }
    }

    // `cd ..` while in an ISO directory — trim the path string, no FAT12 lookup needed.
    if name_input == b".." {
        if vfs::try_iso9660_absolute(cur_path).is_some() {
            let new_path: &[u8] = match cur_path.iter().rposition(|&b| b == b'/') {
                Some(0) | None => b"/",
                Some(i)        => &cur_path[..i],
            };
            if let Some(mut c) = config::SYSTEM_CONFIG.try_lock() {
                c.set_path(new_path, 0);
            }
            return;
        }
    }

    // `cd ..` — follow the FAT12 '..' entry and trim the display path.
    if name_input == b".." {
        if cur_path == b"/" { return; }
        let parent_cluster = fs.find_entry(cwd, &fat83(b".."))
            .map_or(0, |e| e.start_cluster);
        let new_path: &[u8] = match cur_path.iter().rposition(|&b| b == b'/') {
            Some(0) | None => b"/",
            Some(i)        => &cur_path[..i],
        };
        if let Some(mut c) = config::SYSTEM_CONFIG.try_lock() {
            c.set_path(new_path, parent_cluster);
        }
        return;
    }

    // Determine FAT12-relative path and base cluster.
    let (rel, base_cluster) = if name_input.starts_with(b"/") {
        match vfs::try_fat12_absolute(name_input) {
            Some(rel) => (rel, 0u16),
            None => { error!("cd: not a known mount point\n"); return; }
        }
    } else {
        (name_input, cwd)
    };

    // Walk multi-component path (handles "foo", "foo/bar", etc.).
    let entry = match fs.resolve_path_from(base_cluster, rel) {
        Some(e) => e,
        None    => { error!("no such directory\n"); return; }
    };
    if entry.attr & 0x10 == 0 {
        error!("not a directory\n");
        return;
    }

    // Build the new display path string.
    let mut new_path_buf = [0u8; 32];
    let new_path_len: usize = if name_input.starts_with(b"/") {
        let n = name_input.len().min(32);
        new_path_buf[..n].copy_from_slice(&name_input[..n]);
        n
    } else {
        // Append component(s) to the current path.
        let base: &[u8] = if cur_path == b"/" { b"" } else { cur_path };
        let mut w = 0usize;
        for &b in base.iter().take(32) { new_path_buf[w] = b; w += 1; }
        if w < 32 { new_path_buf[w] = b'/'; w += 1; }
        for &b in name_input.iter().take(32usize.saturating_sub(w)) {
            new_path_buf[w] = b; w += 1;
        }
        w
    };

    if let Some(mut c) = config::SYSTEM_CONFIG.try_lock() {
        c.set_path(&new_path_buf[..new_path_len], entry.start_cluster);
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

/// Prints contents of a directory.  Optional argument selects the path; defaults to CWD.
fn cmd_dir(args: &[u8]) {
    let (path_arg, _) = keyboard::split_cmd(args);

    // Snapshot current path and cluster (needed whether or not a path_arg is given).
    let (cur_path_buf, cur_path_len, cwd_cluster) = {
        match config::SYSTEM_CONFIG.try_lock() {
            Some(c) => {
                let p = c.get_path();
                let mut buf = [0u8; 64];
                let len = p.len().min(64);
                buf[..len].copy_from_slice(&p[..len]);
                (buf, len, c.get_path_cluster())
            }
            None => { let mut b = [0u8; 64]; b[0] = b'/'; (b, 1, 0) }
        }
    };
    let cur_path = &cur_path_buf[..cur_path_len];

    // Build the absolute path we actually want to list.
    let mut abs_buf = [0u8; 64];
    let abs_path: &[u8] = if path_arg.is_empty() {
        cur_path
    } else if path_arg.starts_with(b"/") {
        path_arg
    } else {
        // Relative: append component(s) to current path.
        let base: &[u8] = if cur_path == b"/" { b"" } else { cur_path };
        let mut w = 0usize;
        for &b in base { if w < 64 { abs_buf[w] = b; w += 1; } }
        if w < 64 { abs_buf[w] = b'/'; w += 1; }
        for &b in path_arg { if w < 64 { abs_buf[w] = b; w += 1; } }
        &abs_buf[..w]
    };

    // Dispatch to ISO9660 if the resolved mount is iso9660.
    if let Some(iso_rel) = vfs::try_iso9660_absolute(abs_path) {
        match Iso9660::probe() {
            None => { error!("iso: device not available\n"); }
            Some(iso) => {
                let entry = if iso_rel.is_empty() {
                    // Root of the ISO mount
                    crate::fs::iso9660::IsoEntry {
                        is_dir: true, lba: iso.root_lba, size: iso.root_size,
                        ..Default::default()
                    }
                } else {
                    match iso.resolve(iso_rel) {
                        Some(e) => e,
                        None => { error!("no such directory\n"); return; }
                    }
                };
                if !entry.is_dir { error!("not a directory\n"); return; }

                let mut entries = [crate::fs::iso9660::IsoEntry::default(); 64];
                let count = iso.list_dir(entry.lba, entry.size, &mut entries);
                for e in &entries[..count] {
                    printb!(&e.name[..e.name_len as usize]);
                    if e.is_dir { print!("/"); }
                    println!();
                }
            }
        }
        return;
    }

    // FAT12 listing.
    let fat_cluster = if path_arg.is_empty() {
        cwd_cluster
    } else {
        // Absolute FAT12 path?
        match vfs::try_fat12_absolute(abs_path) {
            Some(rel) => {
                let floppy = Floppy;
                match Filesystem::new(&floppy) {
                    Ok(fs) => {
                        match fs.resolve_path_from(0, rel) {
                            Some(e) if e.attr & 0x10 != 0 => e.start_cluster,
                            _ => { error!("no such directory\n"); return; }
                        }
                    }
                    Err(e) => { error!(e); error!(); return; }
                }
            }
            None => {
                // Relative FAT12 path.
                let floppy = Floppy;
                match Filesystem::new(&floppy) {
                    Ok(fs) => {
                        match fs.resolve_path_from(cwd_cluster, path_arg) {
                            Some(e) if e.attr & 0x10 != 0 => e.start_cluster,
                            _ => { error!("no such directory\n"); return; }
                        }
                    }
                    Err(e) => { error!(e); error!(); return; }
                }
            }
        }
    };

    let floppy = Floppy;
    match Filesystem::new(&floppy) {
        Ok(fs) => unsafe {
            fs.for_each_entry(fat_cluster, |entry| {
                if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.name[0] == 0xFF {
                    return;
                }
                fs.print_name(entry);
            });
        },
        Err(e) => { error!(e); error!(); }
    }
}

/// Echos the arguments back to the display.
fn cmd_echo(args: &[u8]) {
    printb!(args);
    println!();
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
        print!(" ", Color::Blue);
        printb!(cmd.name);
        print!(": ", Color::White);
        printb!(cmd.description);
        println!();
    }
}

fn cmd_hlt(_args: &[u8]) {
    print!("\n\n --- Shutting down the system", Color::DarkCyan);

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

fn cmd_kill(args: &[u8]) {
    if args.is_empty() {
        warn!("usage: kill <pid>\n");
        return;
    }

    let (first, _) = keyboard::split_cmd(args);

    if let Some(pid) = parse_u64(first) {
        print!("Killing PID ", Color::White);
        printn!(pid);
        println!();

        unsafe {
            crate::task::scheduler::kill(pid as usize);
        }
    } else {
        error!("invalid PID lmao\n");
    }
}

/// Experimental command function to evaluate the current TUI rendering options.
/*fn cmd_menu(_args: &[u8]) {
    // Set the labels
    let mut label1 = Label {
        x: 0,
        y: 0,
        text: "Play",
        attr: 0x0F,
    };
    let mut label2 = Label {
        x: 0,
        y: 2,
        text: "Scores",
        attr: 0x0F,
    };
    let mut label3 = Label {
        x: 0,
        y: 4,
        text: "Quit",
        attr: 0x0F,
    };

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
}*/

/// Creates new subdirectory in the current directory.
fn cmd_mkdir(args: &[u8]) {
    if args.is_empty() || args.len() > 11 {
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

            let path_cluster = {
                if let Some(c) = config::SYSTEM_CONFIG.try_lock() {
                    c.get_path_cluster()
                } else {
                    0
                }
            };

            fs.create_subdirectory(&filename, path_cluster);
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

fn cmd_mount(_args: &[u8]) {
    if let Some(vfs_table) = vfs::VFS.try_lock() {
        let count = vfs_table.count();
        if count == 0 {
            println!("No mounts.");
            return;
        }
        for i in 0..count {
            if let Some(m) = vfs_table.get(i) {
                let path = &m.path[..m.path_len];
                let fstype: &[u8] = match m.fs_type {
                    vfs::FsType::Root    => b"rootfs",
                    vfs::FsType::Fat12   => b"fat12",
                    vfs::FsType::Iso9660 => b"iso9660",
                    vfs::FsType::None    => b"none",
                };
                printb!(path);
                print!(" (");
                printb!(fstype);
                print!(")\n");
            }
        }
    } else {
        error!("mount: VFS lock unavailable\n");
    }
}

/// Renames given <old_name> to <new_name> in the current directory.
fn cmd_mv(args: &[u8]) {
    if args.is_empty() {
        warn!("Usage: mv <old> <new>\n");
        return;
    }
    let (old, new) = split_cmd(args);
    if old.is_empty() || new.is_empty() {
        warn!("Usage: mv <old> <new>\n");
        return;
    }
    let cwd = config::SYSTEM_CONFIG.try_lock().map_or(0, |c| c.get_path_cluster());
    let floppy = Floppy::init();
    match Filesystem::new(&floppy) {
        Ok(fs) => {
            fs.rename_file(cwd, &fat83(old), &fat83(new));
        }
        Err(e) => { error!(e); error!(); }
    }
}

/// Prints the contents of a file.
fn cmd_read(args: &[u8]) {
    if args.is_empty() {
        warn!("Usage: read <filename>\n");
        return;
    }
    let (name_input, _) = keyboard::split_cmd(args);

    // Build absolute path when name_input is relative and CWD is known.
    let (cur_path_buf, cur_path_len) = {
        match config::SYSTEM_CONFIG.try_lock() {
            Some(c) => {
                let p = c.get_path();
                let mut buf = [0u8; 64];
                let len = p.len().min(64);
                buf[..len].copy_from_slice(&p[..len]);
                (buf, len)
            }
            None => { let mut b = [0u8; 64]; b[0] = b'/'; (b, 1) }
        }
    };
    let cur_path = &cur_path_buf[..cur_path_len];

    let mut abs_buf = [0u8; 64];
    let abs_path: &[u8] = if name_input.starts_with(b"/") {
        name_input
    } else {
        let base: &[u8] = if cur_path == b"/" { b"" } else { cur_path };
        let mut w = 0usize;
        for &b in base { if w < 64 { abs_buf[w] = b; w += 1; } }
        if w < 64 { abs_buf[w] = b'/'; w += 1; }
        for &b in name_input { if w < 64 { abs_buf[w] = b; w += 1; } }
        &abs_buf[..w]
    };

    // ISO9660 dispatch.
    if let Some(iso_rel) = vfs::try_iso9660_absolute(abs_path) {
        match Iso9660::probe() {
            None => { error!("iso: device not available\n"); }
            Some(iso) => {
                match iso.resolve(iso_rel) {
                    None => { error!("no such file\n"); }
                    Some(e) if e.is_dir => { error!("not a file\n"); }
                    Some(e) => {
                        // Read up to 4096 bytes (kernel stack limit — use a reasonable cap).
                        let mut buf = [0u8; 4096];
                        let n = iso.read_file(&e, &mut buf);
                        print!("File contents:\n", Color::DarkYellow);
                        printb!(&buf[..n]);
                        println!();
                    }
                }
            }
        }
        return;
    }

    // FAT12 dispatch.
    let (rel, cwd) = match vfs::try_fat12_absolute(name_input) {
        Some(rel) => (rel, 0u16),
        None      => (name_input, config::SYSTEM_CONFIG.try_lock().map_or(0, |c| c.get_path_cluster())),
    };
    let floppy = Floppy::init();
    match Filesystem::new(&floppy) {
        Ok(fs) => {
            let name83 = fat83(rel);
            match fs.find_entry(cwd, &name83) {
                Some(entry) if entry.attr & 0x10 == 0 => {
                    let mut buf = [0u8; 4096];
                    fs.read_file(entry.start_cluster, &mut buf);
                    print!("File contents:\n", Color::DarkYellow);
                    let len = (entry.file_size as usize).min(buf.len());
                    printb!(&buf[..len]);
                    println!();
                }
                Some(_) => { error!("not a file\n"); }
                None     => { error!("no such file\n"); }
            }
        }
        Err(e) => { error!(e); error!(); }
    }
}

/// Removes a file in the current directory according to the input.
fn cmd_rm(args: &[u8]) {
    if args.is_empty() {
        warn!("Usage: rm <filename>\n");
        return;
    }
    let (name_input, _) = keyboard::split_cmd(args);
    let (rel, cwd) = match vfs::try_fat12_absolute(name_input) {
        Some(rel) => (rel, 0u16),
        None      => (name_input, config::SYSTEM_CONFIG.try_lock().map_or(0, |c| c.get_path_cluster())),
    };
    let floppy = Floppy::init();
    match Filesystem::new(&floppy) {
        Ok(fs) => { fs.delete_file(cwd, &fat83(rel)); }
        Err(e) => { error!(e); error!(); }
    }
}

fn cmd_run(args: &[u8]) {
    if args.is_empty() || args.len() > 12 {
        warn!("usage: run <binary name>\n");
        return;
    }

    // This split_cmd invocation trims the b'\0' tail from the input args.
    let (filename_input, _) = keyboard::split_cmd(args);

    if filename_input.is_empty() || filename_input.len() > 12 {
        warn!("Usage: run <binary name>\n");
        return;
    }

    super::elf::run_elf(filename_input, args, super::elf::RunMode::Foreground);
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

fn cmd_ts(_args: &[u8]) {
    unsafe {
        crate::task::scheduler::list_processes();
    }
}

fn cmd_uptime(_args: &[u8]) {
    let total = time::acpi::get_uptime_seconds();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;

    print!("Uptime: ");
    printn!(h);
    print!(" hour");
    if h != 1 { print!("s"); }
    print!(" ");
    printn!(m);
    print!(" minute");
    if m != 1 { print!("s"); }
    print!(" ");
    printn!(s);
    print!(" second");
    if s != 1 { print!("s"); }
    println!();
}

/// Prints system information set, mainly version and name.
fn cmd_ver(_args: &[u8]) {
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

            if filename.is_empty() || content.is_empty() {
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

            let path_cluster = {
                if let Some(c) = config::SYSTEM_CONFIG.try_lock() {
                    c.get_path_cluster()
                } else {
                    0
                }
            };

            fs.write_file(path_cluster, &name, content);
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}
