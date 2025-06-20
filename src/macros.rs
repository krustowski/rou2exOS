use crate::vga::writer::{Color, Writer};

pub static mut WRITER: Option<Writer> = None;

pub fn init_writer() {
    unsafe {
        WRITER = Some(Writer::new());
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
        $crate::print!($arg, Color::Yellow, Color::Black);
    };
}

#[macro_export]
macro_rules! print {
    ($arg:expr) => {
        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                writer.set_color(Color::White, Color::Black);
                writer.write_str_raw($arg);
            }
        }
    };
    ($arg:expr, $fg:expr) => {
        use crate::vga::writer::Color;

        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                writer.set_color($fg, Color::Black);
                writer.write_str_raw($arg);
            }
        }
    };
    ($arg:expr, $fg:expr, $bg:expr) => ({
        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                writer.set_color($fg, $bg);
                writer.write_str_raw($arg);
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

