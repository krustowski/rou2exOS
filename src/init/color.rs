//

use crate::video::vga::Color;

pub fn color_demo() {
    let colors: [u8; 16] = [
        0x0, 0x1, 0x2, 0x3,
        0x4, 0x5, 0x6, 0x7,
        0x8, 0x9, 0xA, 0xB,
        0xC, 0xD, 0xE, 0xF,
    ];

    print!("Color test:\n");

    let mut col = 0;
    for &color in colors.iter() {
        if col % 8 == 0 {
            // Render new row of colours
            print!("\n");
            col = 0;
        }

        print!(" ", Color::Black, color);
        print!(" ", Color::Black, color);
        print!(" ", Color::Black, Color::Black);

        col += 1;
    }

    print!("\n\n");
}
