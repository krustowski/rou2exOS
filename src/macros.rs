use spin::{mutex::Mutex};
use core::sync::atomic::{AtomicBool, Ordering};
use crate::vga::writer::{Writer};

pub static mut WRITER: Option<Mutex<Writer>> = None;
static WRITER_INIT: AtomicBool = AtomicBool::new(false);

/// Initializes the global Writer instance
pub fn init_writer() {
    if WRITER_INIT.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        let writter = Writer::new();
        unsafe {
            WRITER = Some(Mutex::new(writter));
        }
    }
}

/// Returns a wrapped Writer instance guarded by Mutex in Option
pub fn get_writer() -> Option<spin::MutexGuard<'static, Writer>> {
    if WRITER_INIT.load(Ordering::Relaxed) {
        unsafe { WRITER.as_ref().map(|m| m.lock()) }
    } else {
        None
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
macro_rules! println {
    () => ($crate::print!("\n"));
    ($arg:expr) => ({
        $crate::print!($arg);
        $crate::print!("\n");
    });
}

#[macro_export]
macro_rules! print {
    ($arg:expr) => {
        if let Some(mut writer) = $crate::macros::get_writer() { 
            writer.set_color(Color::White, Color::Black);
            writer.write_str_raw($arg);
        }

    };
    ($arg:expr, $fg:expr) => {
        use crate::vga::writer::Color;

        if let Some(mut writer) = $crate::macros::get_writer() { 
            writer.set_color($fg, Color::Black);
            writer.write_str_raw($arg);
        }
    };
    ($arg:expr, $fg:expr, $bg:expr) => ({
        if let Some(mut writer) = $crate::macros::get_writer() { 
            writer.set_color($fg, $bg);
            writer.write_str_raw($arg);
        }
    });
}

