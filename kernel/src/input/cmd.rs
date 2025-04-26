use crate::vga;

const KERNEL_VERSION: &[u8] = b"0.2.0";

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
        name: b"shutdown",
        description: b"shuts down the system",
        function: cmd_shutdown,
    },
    Command {
        name: b"version",
        description: b"prints the kernel version",
        function: cmd_version,
    },
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
    beep(5000);

    for _ in 0..3_000_000 {
        unsafe { core::arch::asm!("nop"); }
    }

    stop_beep();
}

fn beep(frequency: u32) {
    let divisor = 1_193_180 / frequency; // PIT runs at 1.19318 MHz

    unsafe {
        // Set PIT to mode 3 (square wave generator)
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x43,
            in("al") 0b10110110u8,
        );

        // Set frequency divisor (low byte first, then high byte)
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x42,
            in("al") (divisor & 0xFF) as u8,
        );
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x42,
            in("al") (divisor >> 8) as u8,
        );

        // Enable speaker
        let mut tmp: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") 0x61,
            out("al") tmp,
        );
        if (tmp & 3) != 3 {
            tmp |= 3;
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x61,
                in("al") tmp,
            );
        }
    }
}

fn stop_beep() {
    // Stop the beep.
    unsafe {
        let mut tmp: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") 0x61,
            out("al") tmp,
        );
        tmp &= !3;
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x61,
            in("al") tmp,
        );
    }
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

    unsafe {
        // ACPI shutdown port (common for Bochs/QEMU/VirtualBox)
        const SLP_TYPA: u16 = 0x2000;
        const SLP_EN: u16 = 1 << 13;

        // Fallback PM1a control port address
        const PM1A_CNT_PORT: u16 = 0x604;

        // Write shutdown command
        let value = SLP_TYPA | SLP_EN;

        core::arch::asm!(
            "out dx, ax",
            in("dx") PM1A_CNT_PORT,
            in("ax") value,
        );
    }

    // Freeze in case of the shutdown failure (no ACPI).
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

fn cmd_version(_args: &[u8], vga_index: &mut isize) {
    vga::write::string(vga_index, b"Version: ", 0x0f);
    vga::write::string(vga_index, KERNEL_VERSION, 0x0f);
    vga::write::newline(vga_index);
}
