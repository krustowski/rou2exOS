use crate::video::{bufmg, sysprint, vga};
//system warning macros etc
#[macro_export]
macro_rules! error {
    () => {
        $crate::print!("\n");
    };
    ($arg:expr $(,)?) => {
        $crate::print!(
            "ERR: ",
            $crate::video::vga::Color::Red,
            $crate::video::vga::Color::Black
        );
        $crate::print!(
            $arg,
            $crate::video::vga::Color::Red,
            $crate::video::vga::Color::Black
        );
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
        $crate::print!(
            $arg,
            $crate::video::vga::Color::Yellow,
            $crate::video::vga::Color::Black
        );
    };
}

//result printing macro
//arg1 is the message, arg2 is the status for it
#[macro_export]
macro_rules! result {
    ($msg:expr, $res: expr) => {
        //key created
        if let Some(mut instance) = $crate::video::sysprint::SYSBUFFER.try_lock() {
            instance.format($msg, $res);
        };
    };
}

