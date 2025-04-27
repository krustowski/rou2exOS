use core;

//
//  PORT HANDLING
//

pub fn read(port: u16) -> u8 {
    let data: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx", 
            in("dx") port, 
            out("al") data
        );
    }
    data
}

/// Writes a byte to a port (needs inline assembly)
pub fn write(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
        );
    }
}


