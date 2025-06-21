use spin::{mutex::Mutex};
use core::sync::atomic::{AtomicBool, Ordering};
use crate::vga::writer::{Writer};

/// Wrapped Writer instance guarded by Mutex.
pub static mut WRITER: Option<Mutex<Writer>> = None;

/// Helper static boolean to ensure that the global Writer instance is created just once.
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

/// Returns a wrapped Writer instance guarded by Mutex in Option. Beware that this invocation locks
/// the Writer instance and all print macros therefore can fail silently.
pub fn get_writer() -> Option<spin::MutexGuard<'static, Writer>> {
    if WRITER_INIT.load(Ordering::Relaxed) {
        unsafe { WRITER.as_ref().map(|m| m.lock()) }
    } else {
        // Not initialized yet
        None
    }
}

//
//  PRINT MACROS
//

/// Macro to render all rows with the currently set ColorCode.
#[macro_export]
macro_rules! clear_screen {
    () => {
        if let Some(mut writer) = $crate::macros::get_writer() {
            writer.clear_screen();
        }
    };
}

/// Prints the error string to screen in red.
#[macro_export]
macro_rules! error {
    () => {
        $crate::print!("\n");
    };
    ($arg:expr $(,)?) => {
        // Set yellow chars on black
        $crate::print!($arg, $crate::vga::writer::Color::Red, $crate::vga::writer::Color::Black);
    };
}

/// Prints the warning string to screen in yellow.
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

/// This macro takes in a reference to byte slice (&[u8]) and prints all its contents to display.
#[macro_export]
macro_rules! printb {
    ($arg:expr) => {
        if let Some(mut writer) = $crate::macros::get_writer() { 
            writer.set_color($crate::vga::writer::Color::White, $crate::vga::writer::Color::Black);
            for b in $arg {
                writer.write_byte(*b);
            }
        }
    };
}

/// Meta macro to include the newline implicitly at the end of provided string.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($arg:expr) => ({
        $crate::print!($arg);
        $crate::print!("\n");
    });
}

/// The main string printing macro. Takes in 1-3 arguments. The first one is always the string to
/// print, and the latter ones are to change the foreground color (and background respectively) of
/// the characters printed to screen.
#[macro_export]
macro_rules! print {
    ($arg:expr) => {
        if let Some(mut writer) = $crate::macros::get_writer() { 
            writer.set_color($crate::vga::writer::Color::White, $crate::vga::writer::Color::Black);
            writer.write_str_raw($arg);
        }
    };
    ($arg:expr, $fg:expr) => {
        if let Some(mut writer) = $crate::macros::get_writer() { 
            writer.set_color($fg, $crate::vga::writer::Color::Black);
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

