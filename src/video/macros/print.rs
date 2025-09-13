//
//  PRINT MACROS
//
//generic print macros
/// Macro to render all rows with the currently set ColorCode.
use crate::video::{vga};
#[macro_export]
macro_rules! clear_screen {
    () => {
        if let Some(mut writer) = $crate::video::vga::get_writer() {
            writer.clear_screen();
        }
    };
}


/// This macro takes in a reference to byte slice (&[u8]) and prints all its contents to display.
#[macro_export]
macro_rules! printb {
    ($arg:expr $(,$col: ident)?) => {
        if let Some(mut writer) = $crate::video::vga::get_writer() { 
            //writer.set_color($crate::vga::writer::Color::White, $crate::vga::writer::Color::Black);
			$(writer.set_color($crate::video::vga::Color::$col, $crate::video::vga::Color::Black);)?
            for b in $arg {
                writer.write_byte(*b);
            }
        }
    };
}

/// Special macro to print u64 numbers as a slice of u8 bytes. why?
#[macro_export]
macro_rules! printn {
    ($arg:expr) => {
        //
        let mut buf = [0u8; 20];
        let mut len = buf.len();

        if $arg == 0 {
            print!("0");
            //return 
        }

        let mut num = $arg;

        while num > 0 {
            len -= 1;
            if let Some(b) = buf.get_mut(len) {
                *b = b'0' + (num % 10) as u8;
            }
            num /= 10;
        }

        printb!(buf.get(len..).unwrap_or(&[]));
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
        if let Some(mut writer) = $crate::video::vga::get_writer() { 
            //writer.set_color($crate::vga::writer::Color::White, $crate::vga::writer::Color::Black);
            writer.write_str_raw($arg);
        }
    };
    ($arg:expr, $fg:expr) => {
        if let Some(mut writer) = $crate::video::vga::get_writer() { 
            writer.set_color_num($fg as u8, $crate::video::vga::Color::Black as u8);
            writer.write_str_raw($arg);
        }
    };
    ($arg:expr, $fg:expr, $bg:expr) => ({
        if let Some(mut writer) = $crate::video::vga::get_writer() { 
            writer.set_color_num($fg as u8, $bg as u8);
            writer.write_str_raw($arg);
        }
    });
}


