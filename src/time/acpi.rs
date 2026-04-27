use crate::init::pit::TICKS_PER_SECOND;

static mut TICK_COUNT: u64 = 0;

/// Called once per PIT interrupt (from `scheduler_schedule`).
pub fn tick() {
    unsafe { TICK_COUNT += 1; }
}

pub fn get_uptime_seconds() -> u64 {
    unsafe { TICK_COUNT / TICKS_PER_SECOND }
}

pub fn get_tick_count() -> u64 {
    unsafe { TICK_COUNT }
}

