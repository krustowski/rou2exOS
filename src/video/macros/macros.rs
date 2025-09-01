//
//  PRINT MACROS
//

/// Macro to render all rows with the currently set ColorCode.
#[macro_export]
macro_rules! clear_screen {
    () => {
        if let Some(mut writer) = $crate::video::vga::get_writer() {
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

/// This macro takes in a reference to byte slice (&[u8]) and prints all its contents to display.
#[macro_export]
macro_rules! printb {
    ($arg:expr) => {
        if let Some(mut writer) = $crate::video::vga::get_writer() { 
            //writer.set_color($crate::vga::writer::Color::White, $crate::vga::writer::Color::Black);
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


#[derive(PartialEq, Copy, Clone)]
pub enum InitResult {
    Unknown,
    Passed,
    Failed,
    Skipped,
}

impl InitResult {
    pub fn format(&self) -> (&[u8; 6], Color) {
        match self {
            InitResult::Unknown => 
                (b"UNKNWN", Color::Cyan),
            InitResult::Passed => 
                (b"  OK  ", Color::Green),
            InitResult::Failed => 
                (b" FAIL ", Color::Red),
            InitResult::Skipped => 
                (b" SKIP ", Color::Yellow),
        }
    }
}

const MAX_MSG_LEN: usize = 60;

pub fn print_result(message: &'static str, result: InitResult) {
    let mut buf = Buffer::new();
    
    buf.append(message.as_bytes());

    for _ in 0..MAX_MSG_LEN - message.len() {
        buf.append(b".");
    }

    buf.append(b" [");
    buf.append(result.format().0);
    buf.append(b"]\n");

    if let Some(slice) = buf.buf.get(..buf.pos) {
        //
        INIT_BUFFER.lock().append(slice);
    }
}

struct Buffer {
    buf: [u8; 1024],
    pos: usize,
}

impl Buffer {
    /// Creates and returns a new instance of Buffer. why multiple instances??
    const fn new() -> Self {
        Self {
            buf: [0u8; BUFFER_SIZE],
            pos: 0,
        }
    }

    /// Adds given byte slice to the buffer at offset of self.pos.
    fn append(&mut self, s: &[u8]) {
        // Take the input length, or the offset
        let len = s.len().min(self.buf.len() - self.pos);

        if let Some(buf) = self.buf.get_mut(self.pos..self.pos + len) {
            if let Some(slice) = s.get(..len) {
                // Copy the slice into buffer at offset of self.pos
                buf.copy_from_slice(slice);
                self.pos += len;
            }
        }
    }

    /// Puts the contents of buf into the printb! macro.
    fn flush(&self) {
        if let Some(buf) = self.buf.get(..self.pos) {
            printb!(buf);
        }
    }
}

