use crate::vga;

pub fn handle(input: &[u8], vga_index: &mut isize) {
    match input {
        b"cls" | b"clear" => {
            vga::screen::clear(vga_index);
        }
        b"rustak" => {
            vga::write::string(vga_index, b"VYPADNI OKAMZITE", 0xc);
            vga::write::newline(vga_index);
        }
        _ => {
            if input.len() == 0 {
                return;
            }

            // Echo back the input
            vga::write::string(vga_index, b"unknown command: ", 0xc);
            vga::write::string(vga_index, input, 0x0f);
            vga::write::newline(vga_index);
        }
    }
}

