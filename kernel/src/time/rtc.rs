pub fn read_rtc_register(reg: u8) -> u8 {
    unsafe {
        // Tell CMOS what address we want
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x70,
            in("al") reg,
        );

        // Read the data
        let value: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") 0x71,
            out("al") value,
        );
        value
    }
}

pub fn read_rtc_full() -> (u16, u8, u8, u8, u8, u8) {
    let mut seconds;
    let mut minutes;
    let mut hours;
    let mut day;
    let mut month;
    let mut year;
    let mut century = 20; // fallback if no CMOS reg 0x32

    loop {
        if (read_rtc_register(0x0A) & 0x80) == 0 {
            seconds = read_rtc_register(0x00);
            minutes = read_rtc_register(0x02);
            hours   = read_rtc_register(0x04);
            day     = read_rtc_register(0x07);
            month   = read_rtc_register(0x08);
            year    = read_rtc_register(0x09);
            let maybe_century = read_rtc_register(0x32);

            if maybe_century != 0 {
                century = bcd_to_bin(maybe_century) as u16;
            }

            break;
        }
    }

    if (read_rtc_register(0x0B) & 0x04) == 0 {
        seconds = bcd_to_bin(seconds);
        minutes = bcd_to_bin(minutes);
        hours   = bcd_to_bin(hours);
        day     = bcd_to_bin(day);
        month   = bcd_to_bin(month);
        year    = bcd_to_bin(year);
    }

    let full_year = century * 100 + year as u16;

    (full_year, month, day, hours, minutes, seconds)
}

fn bcd_to_bin(value: u8) -> u8 {
    (value & 0x0F) + ((value >> 4) * 10)
}

