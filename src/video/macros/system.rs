use crate::video::{vga, sysprint, bufmg};
//system warning macros etc
#[macro_export]
macro_rules! error {
    () => {
        $crate::print!("\n");
    };
    ($arg:expr $(,)?) => {
        // Set yellow chars on black
        $crate::print!($arg, $crate::video::vga::Color::Red, $crate::video::vga::Color::Black);
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
        $crate::print!($arg, $crate::video::vga::Color::Yellow, $crate::video::vga::Color::Black);
    };
}




//result printing macro
//arg1 is the message, arg2 is the status for it
#[macro_export]
macro_rules! result {
	() => {
		$crate::print!("\n");
	}; 
	//could be problematic?
	($arg:expr) => {
		//key created
		if let Some(mut instance) = $crate::video::sysprint::SysBuffer.try_lock() {
			instance.append($arg.as_bytes());
			instance.flush();
			//make this arg to a temporary value

		}




	};
	
}