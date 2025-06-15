use core;

pub fn beep(frequency: u32) {
    let divisor = 1_193_180 / frequency; // PIT runs at 1.19318 MHz

    unsafe {
        // Set PIT to mode 3 (square wave generator)
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x43,
            in("al") 0b10110110u8,
        );

        // Set frequency divisor (low byte first, then high byte)
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x42,
            in("al") (divisor & 0xFF) as u8,
        );
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x42,
            in("al") (divisor >> 8) as u8,
        );

        // Enable speaker
        let mut tmp: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") 0x61,
            out("al") tmp,
        );
        if (tmp & 3) != 3 {
            tmp |= 3;
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x61,
                in("al") tmp,
            );
        }
    }
}

pub fn stop_beep() {
    // Stop the beep.
    unsafe {
        let mut tmp: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") 0x61,
            out("al") tmp,
        );
        tmp &= !3;
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x61,
            in("al") tmp,
        );
    }
}

