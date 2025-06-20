use spin::Mutex;
use crate::vga::writer::{Writer};

pub static mut WRITER: Option<Mutex<Writer>> = None;

pub fn init_writer() {
    unsafe {
        WRITER = Some(Mutex::new(Writer::new()));
    }
}

#[macro_export]
macro_rules! error {
    () => {
        $crate::print!("\n");
    };
    ($arg:expr $(,)?) => {
        use crate::vga::writer::Color;

        // Set yellow chars on black
        $crate::print!($arg, Color::Red, Color::Black);
    };
}

#[macro_export]
macro_rules! warn {
    () => {
        $crate::print!("\n");
    };
    ($arg:expr $(,)?) => {
        // Set yellow chars on black
        $crate::print!($arg, $crate::vga::writer::Color::Yellow, $crate::vga::writer::Color::Black);
    };
}

#[macro_export]
macro_rules! print {
    ($arg:expr) => {
        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                let mut guard = writer.lock();
                guard.set_color(Color::White, Color::Black);
                guard.write_str_raw($arg);
            }
        }
    };
    ($arg:expr, $fg:expr) => {
        use crate::vga::writer::Color;

        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                let mut guard = writer.lock();
                guard.set_color($fg, Color::Black);
                guard.write_str_raw($arg);
            }
        }
    };
    ($arg:expr, $fg:expr, $bg:expr) => ({
        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                let mut guard = writer.lock();
                guard.set_color($fg, $bg);
                guard.write_str_raw($arg);
            }
        }
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($arg:expr) => ({
        $crate::print!("\n");
        $crate::print!($arg);
    });
}

