use crate::{input::keyboard::move_cursor, vga::buffer::VGA_BUFFER};
use crate::input::keyboard::{keyboard_read_scancode};
use crate::app::snake::{engine::run, score::{load_high_scores_fat12, render_scores_window}};

const WIDTH: isize = 80;
// const HEIGHT: isize = 25;

const KEY_UP: u8 = 0x48;
const KEY_DOWN: u8 = 0x50;
// const KEY_LEFT: u8 = 0x4B;
// const KEY_RIGHT: u8 = 0x4D;

const KEY_ESC: u8 = 0x01;
const KEY_ENTER: u8 = 0x1C;

const MENU_WINDOW_X: usize = 26;
const MENU_WINDOW_Y: usize = 6;
const MENU_WINDOW_WIDTH: usize = 27;
const MENU_WINDOW_HEIGHT: usize = 12;

pub static mut SELECTED: usize = 0;

pub fn menu_loop() {
    move_cursor(30, 0);
    clear_screen!();

    let menu = ["New game", "High scores", "Exit to shell"];

    draw_window(MENU_WINDOW_X, MENU_WINDOW_Y, MENU_WINDOW_WIDTH, MENU_WINDOW_HEIGHT, Some("Snake"));

    loop {
        unsafe {
            let scancode = keyboard_read_scancode();

            match scancode {
                KEY_DOWN => {
                    SELECTED = (SELECTED + 1) % menu.len();
                }
                KEY_UP => {
                    if SELECTED == 0 {
                        SELECTED = menu.len() - 1;
                    } else {
                        SELECTED -= 1;
                    }
                }
                KEY_ENTER => {
                    if handle_enter() {
                        return;
                    }
                }
                KEY_ESC => {
                    clear_screen!();
                    return;
                }
                _ => {}
            }
        }

        draw_menu(32, 9, &menu);
    }
}

fn handle_enter() -> bool {
    unsafe {
        match SELECTED {
            0 => {
                clear_screen!();
                run();

                clear_screen!();
                draw_window(MENU_WINDOW_X, MENU_WINDOW_Y, MENU_WINDOW_WIDTH, MENU_WINDOW_HEIGHT, Some("Snake"));
            }
            1 => {
                if let Some(scores) = load_high_scores_fat12() {
                    clear_screen!();

                    SELECTED = 0;
                    render_scores_window(&scores);

                    clear_screen!();
                    draw_window(MENU_WINDOW_X, MENU_WINDOW_Y, MENU_WINDOW_WIDTH, MENU_WINDOW_HEIGHT, Some("Snake"));
                }
                //
            }
            _ => {
                clear_screen!();
                SELECTED = 0;
                //move_cursor_index(vga_index);
                return true;
            }
        }
    }
    false
}

// Draw the window frame with a title
pub fn draw_window(x: usize, y: usize, width: usize, height: usize, title: Option<&str>) {
    let attr = 0x0E; // white on black

    // Corners
    write_char(x, y, 0xC9, attr);                                  // ╔
    write_char(x + width - 1, y, 0xBB, attr);                   // ╗
    write_char(x, y + height - 1, 0xC8, attr);                  // ╚
    write_char(x + width - 1, y + height - 1, 0xBC, attr);   // ╝

    // Horizontal borders
    for i in 1..(width - 1) {
        write_char(x + i, y, 0xCD, attr);                       // ═
        write_char(x + i, y + height - 1, 0xCD, attr);       // ═
    }

    // Vertical borders
    for i in 1..(height - 1) {
        write_char(x, y + i, 0xBA, attr);                       // ║
        write_char(x + width - 1, y + i, 0xBA, attr);        // ║
    }

    // Optional centered title
    if let Some(title) = title {
        let start = x + (width - title.len()) / 2;

        write_char(start - 2, y, b'[', 0x0E);
        write_char(start - 1, y, b' ', 0x0E);

        let mut j = 0;
        for (i, byte) in title.bytes().enumerate() {
            write_char(start + i, y, byte, 0x0E); // yellow on blue
            j += 1;
        }

        write_char(start + j,     y, b' ', 0x0E);
        write_char(start + j + 1, y, b']', 0x0E);
    }
}

// Write a character at (x, y) with a color attribute
fn write_char(x: usize, y: usize, chr: u8, attr: u8) {
    let offset = 2 * (y * WIDTH as usize + x);
    unsafe {
        core::ptr::write_volatile(VGA_BUFFER.add(offset), chr);
        core::ptr::write_volatile(VGA_BUFFER.add(offset + 1), attr);
    }
}

pub fn draw_menu(x: usize, y: usize, items: &[&str]) {
    for (i, &item) in items.iter().enumerate() {
        for (j, byte) in item.bytes().enumerate() {

            unsafe {
                // Write selector arrow
                if i == SELECTED {
                    write_char(x - 2, y + i * 2, b'\x1A', 0x0E); // arrow
                } else {
                    write_char(x - 2, y + i * 2, b' ', 0x07);
                }
            }

            let offset = 2 * ((y + i * 2) * WIDTH as usize + x + j);
            unsafe {
                core::ptr::write_volatile(
                    VGA_BUFFER.add(offset),
                    byte,
                );
                core::ptr::write_volatile(
                    VGA_BUFFER.add(offset + 1),
                    if i == SELECTED {
                        0xE0 // Yellow background, black text
                    } else {
                        0x07 // Light grey on black
                    },
                );
            }
        }
    }
}
