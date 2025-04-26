use crate::vga;

struct Command {
    name: &'static [u8],
    description: &'static [u8],
    function: fn(args: &[u8], vga_index: &mut isize),
}

static COMMANDS: &[Command] = &[
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
];

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
//  COMMAND FUNCTIONS
//

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

