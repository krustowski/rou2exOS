use crate::video::vga::Color;

pub fn ascii_art() {
    print!("                 ____            ___  _____ \n", Color::Green);
    print!(" _ __ ___  _   _|___ \\ _____  __/ _ \\/ ____| \n", Color::Green);
    print!("| '__/ _ \\| | | | __) / _ \\ \\/ / | | \\___ \\\n", Color::Green);
    print!("| | | (_) | |_| |/ __/  __/>  <| |_| |___) |\n", Color::Green);
    print!("|_|  \\___/ \\__,_|_____\\___/_/\\_\\____/|____/\n\n", Color::Green);

    // Set the fg color back to white on return
    print!("", Color::White);
}
