use crate::vga::writer::Writer;

pub static mut WRITER: Option<Writer> = None;

pub fn init_writer() {
    unsafe {
        WRITER = Some(Writer::new());
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        unsafe {
            if let Some(writer) = &mut $crate::macros::WRITER {
                let _ = write!(writer, $($arg)*);
            }
        }
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::print!("{}\n", format_args!($($arg)*));
    });
}

/*#[macro_export]
macro_rules! debug {
    () => {$crate::println!()};
    ($($arg:tt)*) => ({
        if debug_enabled() {
            $crate::print!("{}\n", format_args!($($arg)*));
        }
    });
}*/
