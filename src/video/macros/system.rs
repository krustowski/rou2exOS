use crate::video::{vga};
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


const MAX_MSG_LEN: usize = 60;

pub fn print_result(message: &'static str, result: vga::Result) {
    let mut buf = vga::SysBuffer::new(); //new buffer instance, make the init initialize this instead?
    
    buf.append(message.as_bytes()); //append as bytes

    for _ in 0..MAX_MSG_LEN - message.len() {
        buf.append(b"."); //write, not going past max
    }

    buf.append(b" [");
    buf.append(result.format().0);
    buf.append(b"]\n");

	//in range ...
    if let Some(slice) = buf.buf.get(..buf.pos) {
        //
        INIT_BUFFER.lock().append(slice); //buffer lock?
    }
}



//arg1 is the message, arg2 is the status for it
#[macro_export]
macro_rules! result {
	() => {
		$crate::print!("\n");
	};
	($str:expr, $res: expr) => {
		let mut buf = vga::SysBuffer;
		let mut len = str.len();
		buf.append(str);
		for _ in 0 MAX_MSG_LEN - len {
			buf.append(b".");
		}
		buf.append(b" [");


		$crate::print!(str);
	};
	
}