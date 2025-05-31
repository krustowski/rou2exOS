use crate::vga;

pub fn color_demo(vga_index: &mut isize) {
    let colors: [u8; 16] = [
        0x0, 0x1, 0x2, 0x3,
        0x4, 0x5, 0x6, 0x7,
        0x8, 0x9, 0xA, 0xB,
        0xC, 0xD, 0xE, 0xF,
    ];

    vga::write::string(vga_index, b"Color test:", vga::buffer::Color::White);
    vga::write::newline(vga_index);

    let mut col = 0;
    for &color in colors.iter() {
        if col % 8 == 0 {
            vga::write::newline(vga_index);
            col = 0;
        }

        unsafe {
            let offset = (col * 2) as isize;
            *vga::buffer::VGA_BUFFER.offset(*vga_index + offset) = b' ';
            *vga::buffer::VGA_BUFFER.offset(*vga_index + offset + 1) =  color << 4 | 0xf;
            *vga::buffer::VGA_BUFFER.offset(*vga_index + offset + 2) = b' ';
            *vga::buffer::VGA_BUFFER.offset(*vga_index + offset + 3) =  color << 4 | 0xf;
            //*VGA_BUFFER.offset(*vga_index + offset + 2) = b'*';
            //*VGA_BUFFER.offset(*vga_index + offset + 3) =  color;
            *vga_index += 4;
        }
        col += 1;
    }

    vga::write::newline(vga_index);
    vga::write::newline(vga_index);

}

