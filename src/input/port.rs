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

/// Read a byte (u8) from port
pub fn read_u8(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
        );
    }
    value
}

/// Write a byte (u8) to port
pub fn write_u8(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
        );
    }
}

/// Read a word (u16) from port
pub fn read_u16(port: u16) -> u16 {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "in ax, dx",
            in("dx") port,
            out("ax") value,
        );
    }
    value
}

/// Write a word (u16) to port
pub fn write_u16(port: u16, value: u16) {
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
        );
    }
}

/// Read a double word (u32) from port
pub fn read_u32(port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") value,
        );
    }
    value
}

/// Write a double word (u32) to port
pub fn write_u32(port: u16, value: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
        );
    }
}

