use crate::fs::fat12::{block::Floppy, fs::Filesystem};
use crate::init::config::PATH_CLUSTER;
use crate::input::cmd;
use crate::input::port;
use crate::init::config::{HOST, USER, get_path};
use crate::video::{self, vga, sysprint};
use crate::video::macros::{print, system};

/// The macimum size of an input to the shell console.
const INPUT_BUFFER_SIZE: usize = 128;

/// Case control keys state booleans.
static mut SHIFT_PRESSED: bool = false;
static mut CAPS_LOCK_ON: bool = false;

/// Internal function to assemble the prompt contents.
fn render_prompt() {
    let path = get_path() as &[u8];

    print!("[", vga::Color::Green);
    printb!(USER);
    print!("@");
    printb!(HOST);
    print!(":");
    print!("", vga::Color::Blue);
    printb!(path);
    print!("", vga::Color::Green);
    print!("] > ");
    print!("", vga::Color::White);
}

//
//  HARDWARE CURSOR HANDLING
//

/// Moves the hardware cursor according to the actual VGA index.
pub fn move_cursor_index(vga_index: &mut isize) {
    let row = (*vga_index / 2) / 80;
    let col = (*vga_index / 2) % 80;

    move_cursor(row as u16, col as u16);
}

/// Move the hardware cursor to position defined as (row, col).
pub fn move_cursor(row: u16, col: u16) {
    let pos: u16 = row * 80 + col; // 80 columns wide

    // Set high byte
    port::write(0x3D4, 0x0E);
    port::write(0x3D5, (pos >> 8) as u8);

    // Set low byte
    port::write(0x3D4, 0x0F);
    port::write(0x3D5, (pos & 0xFF) as u8);
}

//
//  KEYBOARD HANDLING
//

/// Keeps the loop until a key is pressed.
pub fn keyboard_wait_read() {
    while port::read(0x64) & 1 == 0 {}
}

/// Returns last scanned code from the keyboard.
pub fn keyboard_read_scancode() -> u8 {
    keyboard_wait_read();
    port::read(0x60)
}

/// Main command shell loop. 
#[unsafe(no_mangle)]
pub extern "C" fn keyboard_loop() -> ! {
    let mut input_buffer = [0u8; INPUT_BUFFER_SIZE];
    let mut input_len = 0;

    let mut ctrl_down = false;

    //print!("\nStarting shell...\n\n");

    // Write prompt
    render_prompt();

    loop {
        let key = keyboard_read_scancode();

        if key & 0x80 != 0 {
            // Key released
            let released = key & 0x7F;
            if released == 0x1D {
                ctrl_down = false;
            }

            // Update the special key state
            scancode_to_ascii(key);
            continue;
        }

        match key {
            // Control key
            0x1D => {
                ctrl_down = true;
                continue;
            }
            // L key
            0x26 => {
                if ctrl_down {
                    clear_screen!();

                    // Clear input buffer
                    input_buffer = [0u8; 128];
                    input_len = 0;

                    render_prompt();
                    continue;
                }
            }
            // Backspace key
            0x0E => {
                handle_backspace(&mut input_len);
                continue;
            }
            // Enter key
            0x1C => {
                // Break the line with a newline
                println!();

                // Extract the input from buffer and hand it to command handler
                let input_slice = input_buffer.get(..input_len).unwrap_or(&[]);
                cmd::handle(input_slice);

                // Clear input buffer
                input_buffer = [0u8; 128];
                input_len = 0;

                // Show new prompt
                render_prompt();
                continue;
            }
            // Tab key
            0x0F => {
                handle_tab_completion(&mut input_buffer, &mut input_len);
                continue;
            }
            _ => {}
        }

        if let Some(ascii) = scancode_to_ascii(key) {
            // If there is room, add to buffer
            if input_len < INPUT_BUFFER_SIZE {
                if let Some(w) = input_buffer.get_mut(input_len) {
                    *w = ascii
                }

                input_len += 1;

                // Draw the ASCII byte to the screen
                printb!(&[ascii]);
            }
        }
    };
}

/// Runs operations when the Backspace key has been pressed.
fn handle_backspace(input_len: &mut usize) {
    if *input_len > 0 {
        *input_len -= 1;

        print!("\r");
    }
}

/// Runs specialized operations related to the Tab key.
fn handle_tab_completion(input_buffer: &mut [u8; INPUT_BUFFER_SIZE], input_len: &mut usize) {
    let mut input_cpy = [0u8; 128];
    input_cpy.copy_from_slice(input_buffer);

    // Split the input to react accordingly
    let (cmd, prefix) = split_cmd(&input_cpy);

    // Just render the help command output
    if prefix.is_empty() {
        cmd::handle(b"help");
        return;
    }

    let floppy = Floppy::init();
    let mut found = false;

    match Filesystem::new(&floppy) {
        Ok(fs) => {
            unsafe {
                if prefix.len() > 8 {
                    return;
                }

                let padded_prefix = pad_prefix(prefix);

                fs.for_each_entry(PATH_CLUSTER, |entry| {
                    if entry.name[0] != 0x00 && entry.name[0] != 0xE5 {

                        let mut clean_name = [0u8; 12];

                        let name_end = entry.name[..8].iter().position(|&c| c == b' ').unwrap_or(8);
                        let ext_end = entry.ext[..3].iter().position(|&c| c == b' ').unwrap_or(3);

                        if name_end > 8 || ext_end > 3 || name_end == 0 {
                            return;
                        }

                        // Copy the file name
                        if let Some(slice) = clean_name.get_mut(..name_end) {
                            if let Some(sl) = entry.name.get(..name_end) {
                                slice.copy_from_slice(sl);
                            }
                        }

                        // TODO: Review this, it is overeng'd to fix the linker panicking errors.
                        if ext_end <= 3 && name_end > 0 && name_end <= 8 && ext_end + name_end <= 12 {
                            // Directories should not contain dots in their name (no extensions)
                            if cmd != b"cd" || ext_end != 0 {
                                clean_name[name_end] = b'.';
                            }

                            // Copy the file extension
                            if let Some(slice) = clean_name.get_mut(name_end + 1..name_end + ext_end + 1) {
                                if let Some(sl) = entry.ext.get(..ext_end) {
                                    slice.copy_from_slice(sl);
                                }
                            }
                        }

                        // Again, this fixes the linker error for panicking code
                        if prefix.len() > 8 {
                            return;
                        }

                        // Match found...
                        if entry.name.starts_with(&padded_prefix[..prefix.len()]) {
                            if cmd == b"cd" && entry.attr & 0x10 == 0 {
                                return;
                            }

                            for _ in 0..prefix.len() {
                                handle_backspace(input_len);
                            }

                            print!("", video::vga::Color::Magenta);
                            printb!(&clean_name);
                            found = true;

                            if cmd.len() > 10 || cmd.len() + 1 > 11 {
                                return;
                            }

                            // Ensure the command name is in the buffer
                            if let Some(slice) = input_buffer.get_mut(..cmd.len()) {
                                slice.copy_from_slice(&cmd[..cmd.len()]);
                            }

                            let clean_name_len = if ext_end > 0 {
                                name_end + 1 + ext_end // include dot
                            } else {
                                name_end
                            }; 

                            // Copy the match into buffer
                            if let Some(slice) = input_buffer.get_mut(cmd.len() + 1..cmd.len() + 1 + clean_name_len) {
                                if name_end + ext_end + 1 > 12 {
                                    return;
                                }
                                slice.copy_from_slice(&clean_name[..clean_name_len]);
                            }

                            // Adjust the input buffer used length
                            *input_len += cmd.len() + 1 + clean_name_len;

                            // Debug: record full input buffer
                            //debugln!(&input_buffer[..*input_len]);
                        }
                    }
                });
            }
        }
        Err(e) => {
            error!(e);
            error!();
        }
    }
}

/// Mapping function for the basic (printable) ASCII characters.
pub fn scancode_to_ascii(sc: u8) -> Option<u8> {
    unsafe {
        match sc {
            // Modifier keys
            0x2A | 0x36 => {
                SHIFT_PRESSED = true;
                return None;
            }
            0xAA | 0xB6 => {
                SHIFT_PRESSED = false;
                return None;
            }
            0x3A => {
                CAPS_LOCK_ON = !CAPS_LOCK_ON;
                return None;
            }

            // Printable keys
            _ => {}
        }

        let shifted = SHIFT_PRESSED;
        let caps = CAPS_LOCK_ON;

        let ch = match sc {
            // Number row (with Shift symbols)
            0x02 => if shifted { b'!' } else { b'1' },
            0x03 => if shifted { b'@' } else { b'2' },
            0x04 => if shifted { b'#' } else { b'3' },
            0x05 => if shifted { b'$' } else { b'4' },
            0x06 => if shifted { b'%' } else { b'5' },
            0x07 => if shifted { b'^' } else { b'6' },
            0x08 => if shifted { b'&' } else { b'7' },
            0x09 => if shifted { b'*' } else { b'8' },
            0x0A => if shifted { b'(' } else { b'9' },
            0x0B => if shifted { b')' } else { b'0' },
            0x0C => if shifted { b'_' } else { b'-' },
            0x0D => if shifted { b'+' } else { b'=' },

            // Letters (Caps Lock + Shift logic)
            0x10..=0x19 | 0x1E..=0x26 | 0x2C..=0x32 => {
                let lower = match sc {
                    0x10 => b'q', 0x11 => b'w', 0x12 => b'e', 0x13 => b'r', 0x14 => b't',
                    0x15 => b'y', 0x16 => b'u', 0x17 => b'i', 0x18 => b'o', 0x19 => b'p',
                    0x1E => b'a', 0x1F => b's', 0x20 => b'd', 0x21 => b'f', 0x22 => b'g',
                    0x23 => b'h', 0x24 => b'j', 0x25 => b'k', 0x26 => b'l',
                    0x2C => b'z', 0x2D => b'x', 0x2E => b'c', 0x2F => b'v',
                    0x30 => b'b', 0x31 => b'n', 0x32 => b'm',
                    _ => return None,
                };
                let upper = lower.to_ascii_uppercase();
                if caps ^ shifted { upper } else { lower }
            }

            // Punctuation
            0x1A => if shifted { b'{' } else { b'[' },
            0x1B => if shifted { b'}' } else { b']' },
            0x27 => if shifted { b':' } else { b';' },
            0x28 => if shifted { b'"' } else { b'\'' },
            0x29 => if shifted { b'~' } else { b'`' },
            0x2B => if shifted { b'|' } else { b'\\' },
            0x33 => if shifted { b'<' } else { b',' },
            0x34 => if shifted { b'>' } else { b'.' },
            0x35 => if shifted { b'?' } else { b'/' },

            // Control keys
            0x0E => 8,         // Backspace
            0x1C => b'\n',     // Enter
            0x39 => b' ',      // Space

            _ => return None,
        };

        Some(ch)
    }
}

/// Pads the provided filename stub to 11 characters to match the FAT12 format.
fn pad_prefix(prefix: &[u8]) -> [u8; 11] {
    let mut padded = [b' '; 11];

    let mut i = 0;
    let mut j = 0;

    while i < prefix.len() && j < 11 {
        if prefix[i] == b'.' {
            j = 8; // Jump to extension part
        } else {
            padded[j] = prefix[i].to_ascii_uppercase(); // FAT stores names uppercase
            j += 1;
        }
        i += 1;
    }

    padded
}

/// Splits a buffer into two parts at the first space (`b' '`),
/// while skipping trailing zeros and handling missing space correctly
pub fn split_cmd(input: &[u8]) -> (&[u8], &[u8]) {
    let len = input.iter().position(|&c| c == 0).unwrap_or(input.len());
    let trimmed = &input[..len];

    // Break the input at first space
    if let Some(pos) = trimmed.iter().position(|&c| c == b' ') {

        let cmd = trimmed.get(..pos).unwrap_or(&[]);
        let mut rest = trimmed.get(pos + 1..).unwrap_or(&[]);

        while rest.first() == Some(&b' ') {
            rest = &rest[1..];
        }
        (cmd, rest)
    } else {
        (trimmed, &[])
    }
}

