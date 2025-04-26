#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

use core::panic::PanicInfo;
//use x86_64::instructions::port::Port;
//let mut port60 = Port::new(0x60);

/// This function is called on panic.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const INPUT_BUFFER_SIZE: usize = 128;

const PROMPT: &[u8] = b"guest@rou2ex:/ > ";

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut vga_index = 0;
    let mut input_buffer = [0u8; INPUT_BUFFER_SIZE];
    let mut input_len = 0;

    // Write prompt
    write_string(&mut vga_index, PROMPT, 0xa);

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
