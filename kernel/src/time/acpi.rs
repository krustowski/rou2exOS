const ACPI_PM_TIMER_PORT: u16 = 0x408; // Hardcoded for now
const PM_TIMER_FREQUENCY_HZ: u64 = 3_579_545; // Hz

static mut LAST_TICKS: u32 = 0;
static mut UPTIME_TICKS: u64 = 0;

pub fn read_pm_timer() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") ACPI_PM_TIMER_PORT,
            out("eax") value,
        );
    }
    value & 0xFFFFFF // 24 bits
}

pub fn update_uptime() {
    let current = read_pm_timer();
    unsafe {
        if current < LAST_TICKS {
            // Wrapped
            UPTIME_TICKS += (0xFFFFFF - LAST_TICKS + current) as u64;
        } else {
            UPTIME_TICKS += (current - LAST_TICKS) as u64;
        }
        LAST_TICKS = current;
    }
}

pub fn get_uptime_seconds() -> u64 {
    update_uptime();
    unsafe { UPTIME_TICKS / PM_TIMER_FREQUENCY_HZ }
}

