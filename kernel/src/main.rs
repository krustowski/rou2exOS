#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
use core::ptr;

//use x86_64::instructions::port::Port;
//let mut port60 = Port::new(0x60);

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const INPUT_BUFFER_SIZE: usize = 128;

const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

const PROMPT: &[u8] = b"guest@rou2ex:/ > ";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut vga_index = 0;
    let mut input_buffer = [0u8; INPUT_BUFFER_SIZE];
    let mut input_len = 0;

    color_demo(&mut vga_index);

    newline(&mut vga_index);
    newline(&mut vga_index);

    // Write prompt
    write_string(&mut vga_index, PROMPT, 0xa);
    move_cursor_index(&mut vga_index);
    scroll_screen(&mut vga_index);

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
                newline(&mut vga_index);

                // Echo back the input
                write_string(&mut vga_index, b"unknown: ", 0xc);
                write_string(&mut vga_index, &input_buffer[..input_len], 0x0f);
                newline(&mut vga_index);

                // Clear input buffer
                input_len = 0;

                // Show new prompt
                write_string(&mut vga_index, PROMPT, 0xa);
                move_cursor_index(&mut vga_index);

                continue;
            }
            0x0E => { // backspace
                if input_len > 0 {
                    input_len -= 1;
                    unsafe {
                        vga_index -= 2; // move cursor back one character
                        *VGA_BUFFER.offset(vga_index) = b' ';
                        *VGA_BUFFER.offset(vga_index + 1) = 0x0f;
                    }
                    move_cursor_index(&mut vga_index);
                }
                continue;
            }
            _ => continue, // ignore unknown keys
        };

        // If we have room, add to buffer
        if input_len < INPUT_BUFFER_SIZE {
            input_buffer[input_len] = c;
            input_len += 1;

            // Draw it on screen
            unsafe {
                *VGA_BUFFER.offset(vga_index) = c;
                *VGA_BUFFER.offset(vga_index + 1) = 0x0f;
                vga_index += 2;
            }
            move_cursor_index(&mut vga_index);
        }
    }
}

fn inb(port: u16) -> u8 {
    let data: u8;
    unsafe {
        core::arch::asm!("in al, dx", in("dx") port, out("al") data);
    }
    data
}

fn keyboard_wait_read() {
    while inb(0x64) & 1 == 0 {}
}

fn keyboard_read_scancode() -> u8 {
    keyboard_wait_read();
    inb(0x60)
}

/// Write a whole string to screen
fn write_string(vga_index: &mut isize, string: &[u8], color: u8) {
    scroll_screen(vga_index);

    for &byte in string {
        unsafe {
            *VGA_BUFFER.offset(*vga_index) = byte;
            *VGA_BUFFER.offset(*vga_index + 1) = color;
            *vga_index += 2;
        }
    }
}

/// Move to a new line
fn newline(vga_index: &mut isize) {
    // VGA 80x25: each line is 80 chars * 2 bytes per char
    *vga_index += (80 * 2) - (*vga_index % (80 * 2));
}

fn scroll_screen(vga_index: &mut isize) {
    if (*vga_index / 2) / 80 < BUFFER_HEIGHT as isize {
        return;
    }

    unsafe {
        // Copy 24 rows * 80 cols * 2 bytes = 3840 bytes
        ptr::copy(
            VGA_BUFFER.offset((BUFFER_WIDTH * 2) as isize), // from row 1
            VGA_BUFFER,
            (BUFFER_WIDTH * (BUFFER_HEIGHT - 1) * 2) as usize, // size: 24 rows
        );

        // Clear last line (row 24)
        let last_line = VGA_BUFFER.offset((BUFFER_WIDTH * (BUFFER_HEIGHT - 1) * 2) as isize);
        for i in 0..BUFFER_WIDTH {
            *last_line.offset((i * 2) as isize) = b' ';
            *last_line.offset((i * 2 + 1) as isize) = 0x07; // Light gray on black
        }
    }

    *vga_index = BUFFER_WIDTH as isize * (BUFFER_HEIGHT as isize - 1) * 2;
}

pub fn color_demo(vga_index: &mut isize) {
    let colors: [u8; 16] = [
        0x0, 0x1, 0x2, 0x3,
        0x4, 0x5, 0x6, 0x7,
        0x8, 0x9, 0xA, 0xB,
        0xC, 0xD, 0xE, 0xF,
    ];

    write_string(vga_index, b"Color test:", 0x0f);
    newline(vga_index);

    let mut col = 0;
    for &color in colors.iter() {
        if col % 8 == 0 {
            newline(vga_index);
            col = 0;
        }

        unsafe {
            let offset = (col * 2) as isize;
            *VGA_BUFFER.offset(*vga_index + offset) = b' ';
            *VGA_BUFFER.offset(*vga_index + offset + 1) =  color << 4 | 0xf;
            *VGA_BUFFER.offset(*vga_index + offset + 2) = b' ';
            *VGA_BUFFER.offset(*vga_index + offset + 3) =  color << 4 | 0xf;
            //*VGA_BUFFER.offset(*vga_index + offset + 2) = b'*';
            //*VGA_BUFFER.offset(*vga_index + offset + 3) =  color;
            *vga_index += 4;
        }
        col += 1;
    }
}

fn move_cursor_index(vga_index: &mut isize) {
    let row = (*vga_index / 2) / 80;
    let col = (*vga_index / 2) % 80;
                                             
    move_cursor(row as u16, col as u16);
}

/// Move the hardware cursor to (row, col)
pub fn move_cursor(row: u16, col: u16) {
    let pos: u16 = row * 80 + col; // 80 columns wide

        // Set high byte
        port_write(0x3D4, 0x0E);
        port_write(0x3D5, (pos >> 8) as u8);

        // Set low byte
        port_write(0x3D4, 0x0F);
        port_write(0x3D5, (pos & 0xFF) as u8);
}

/// Writes a byte to a port (needs inline assembly)
fn port_write(port: u16, value: u8) {
    unsafe {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
    );
    }
}
