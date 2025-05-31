use crate::vga::writer::Writer;

static mut WRITER: Option<Writer> = None;

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
            if let Some(writer) = &mut $crate::WRITER {
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


