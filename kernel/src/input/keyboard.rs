use crate::vga;
use crate::input::cmd;
use crate::input::port;
use crate::init::config::{HOST, PATH, USER, get_path};

const INPUT_BUFFER_SIZE: usize = 128;

fn prompt(vga_index: &mut isize) {
    let path = get_path() as &[u8];

    vga::write::string(vga_index, b"[", vga::buffer::Color::Green);
    vga::write::string(vga_index, USER, vga::buffer::Color::Green);
    vga::write::string(vga_index, b"@", vga::buffer::Color::Green);
    vga::write::string(vga_index, HOST, vga::buffer::Color::Green);
    vga::write::string(vga_index, b":", vga::buffer::Color::Green);
    vga::write::string(vga_index, path, vga::buffer::Color::Blue);
    vga::write::string(vga_index, b"] > ", vga::buffer::Color::Green);
}

//use x86_64::instructions::port::Port;
//let mut port60 = Port::new(0x60);

//
//  HARDWARE CURSOR HANDLING
//

pub fn move_cursor_index(vga_index: &mut isize) {
    let row = (*vga_index / 2) / 80;
    let col = (*vga_index / 2) % 80;

    move_cursor(row as u16, col as u16);
}

/// Move the hardware cursor to (row, col)
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

fn keyboard_wait_read() {
    while port::read(0x64) & 1 == 0 {}
}

fn keyboard_read_scancode() -> u8 {
    keyboard_wait_read();
    port::read(0x60)
}

pub fn keyboard_loop(vga_index: &mut isize) {
    let mut input_buffer = [0u8; INPUT_BUFFER_SIZE];
    let mut input_len = 0;

    vga::write::newline(vga_index);
    vga::write::string(vga_index, b"Starting prompt...", vga::buffer::Color::White);
    vga::write::newline(vga_index);
    vga::write::newline(vga_index);

    // Write prompt
    prompt(vga_index);
    move_cursor_index(vga_index);
    vga::screen::scroll(vga_index);

    loop {
        // Wait for a keypress
        let scancode = keyboard_read_scancode();

        if scancode & 0x80 != 0 {
            continue;
        }

        // VERY basic scancode to ASCII mapping (only letters a-z)
        let c = match scancode {
            0x1E => b'a',
            0x30 => b'b',
            0x2E => b'c',
            0x20 => b'd',
            0x12 => b'e',
            0x21 => b'f',
            0x22 => b'g',
            0x23 => b'h',
            0x17 => b'i',
            0x24 => b'j',
            0x25 => b'k',
            0x26 => b'l',
            0x32 => b'm',
            0x31 => b'n',
            0x18 => b'o',
            0x19 => b'p',
            0x10 => b'q',
            0x13 => b'r',
            0x1F => b's',
            0x14 => b't',
            0x16 => b'u',
            0x2F => b'v',
            0x11 => b'w',
            0x2D => b'x',
            0x15 => b'y',
            0x2C => b'z',
            0x39 => b' ', // spacebar
            0x1C => {
                // ENTER key pressed
                vga::write::newline(vga_index);

                let input_slice = input_buffer.get(..input_len).unwrap_or(&[]);
                cmd::handle(input_slice, vga_index);

                // Clear input buffer
                input_len = 0;

                // Show new prompt
                prompt(vga_index);
                move_cursor_index(vga_index);

                continue;
            }
            0x0E => { // backspace
                if input_len > 0 {
                    input_len -= 1;
                    unsafe {
                        *vga_index -= 2; // move cursor back one character
                        *vga::buffer::VGA_BUFFER.offset(*vga_index) = b' ';
                        *vga::buffer::VGA_BUFFER.offset(*vga_index + 1) = vga::buffer::Color::White as u8;
                    }
                    move_cursor_index(vga_index);
                }
                continue;
            }
            _ => continue, // ignore unknown keys
        };

        // If we have room, add to buffer
        if input_len < INPUT_BUFFER_SIZE {
            if let Some(w) = input_buffer.get_mut(input_len) {
                *w = c
            }

            input_len += 1;

            // Draw it on screen
            unsafe {
                *vga::buffer::VGA_BUFFER.offset(*vga_index) = c;
                *vga::buffer::VGA_BUFFER.offset(*vga_index + 1) = vga::buffer::Color::White as u8;
                *vga_index += 2;
            }
            move_cursor_index(vga_index);
        }
    }
}

