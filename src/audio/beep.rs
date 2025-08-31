use core;

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") val);
    val
}

pub fn beep(freq: u32) {
    let divisor = 1_193_180 / freq;

    unsafe {
        outb(0x43, 0b10110110);
        outb(0x42, (divisor & 0xFF) as u8);
        outb(0x42, (divisor >> 8) as u8);
    }

    unsafe {
        // Enable speaker (bits 0 and 1 on port 0x61)
        core::arch::asm!(
            "in al, dx",
            "or al, 3",
            "out dx, al",
            in("dx") 0x61,
            out("al") _,
        );
    }
}

pub fn stop_beep() {
    unsafe {
        core::arch::asm!(
            "in al, dx",
            "and al, 0xFC", // clear bits 0 and 1
            "out dx, al",
            in("dx") 0x61,
            out("al") _,
        );
    }
}

