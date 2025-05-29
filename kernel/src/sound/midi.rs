use super::beep::{beep, stop_beep};

pub static mut MIDI_FREQ_TABLE: [u16; 128] = [0; 128];

pub fn wait_millis(ms: u16) {
    for _ in 0..(ms as u32 * 1_000) {
        unsafe { core::arch::asm!("nop") };
    }
}

pub fn init_midi_freq_table() {
    for i in 0..128 {
        unsafe {
            MIDI_FREQ_TABLE[i] = midi_note_to_freq(i as u8);
        }
    }
}

fn midi_note_to_freq(note: u8) -> u16 {
    const BASE_FREQ: u32 = 440;
    let n = note as i32 - 69;
    let multiplier = pow2(n);

    let hz = ((BASE_FREQ as u64 * multiplier) >> 20) as u16;
    if hz == 0 { 1 } else { hz } // ← avoid zero frequency
}

fn pow2(n: i32) -> u64 {
    const BASE: u64 = 1 << 20; // Q12.20
    const SEMI_UP: u64 = 108579;   // ≈ 2^(1/12) * BASE
    const SEMI_DOWN: u64 = 94387;  // ≈ 2^(-1/12) * BASE

    let mut result = BASE;

    if n > 0 {
        for _ in 0..n {
            result = (result * SEMI_UP) >> 20;
        }
    } else {
        for _ in 0..(-n) {
            result = (result * SEMI_DOWN) >> 20;
        }
    }

    result
}

pub static MELODY: &[(u8, u16)] = &[
    (41, 237),
    (41, 117),
    (53, 117),
    (41, 117),
    (41, 117),
    (49, 357),
    (44, 117),
    (56, 117),
    (44, 117),
    (44, 117),
    (46, 357),
    (41, 237),
    (41, 117),
    (53, 117),
    (41, 117),
    (41, 117),
    (49, 357),
    (44, 117),
    (56, 117),
    (44, 117),
    (44, 117),
    (46, 357),
    (41, 237),
    (41, 117),
    (53, 117),
    (41, 117),
    (41, 117),
    (49, 357),
    (44, 117),
    (56, 117),
    (44, 117),
    (44, 117),
    (46, 357),
    (41, 237),
    (41, 117),
    (53, 117),
    (41, 117),
    (41, 117),
    (49, 357),
    (44, 117),
    (56, 117),
    (44, 117),
    (44, 117),
    (46, 357),
    (53, 237),
    (53, 117),
    (65, 117),
    (53, 117),
    (53, 117),
    (61, 357),
    (56, 117),
    (68, 117),
    (56, 117),
    (56, 117),
    (56, 117),
    (58, 357),
    (53, 237),
    (53, 117),
    (65, 117),
    (53, 117),
    (53, 117),
    (61, 357),
    (56, 117),
    (68, 117),
    (56, 117),
    (56, 117),
    (58, 357),
    (53, 237),
    (53, 117),
    (65, 117),
    (53, 117),
    (53, 117),
    (61, 357),
    (56, 117),
    (68, 117),
    (56, 117),
    (56, 117),
    (56, 117),
    (56, 117),
    (58, 357),
    (53, 237),
    (53, 117),
    (65, 117),
    (53, 117),
    (53, 117),
    (61, 357),
    (56, 117),
    (68, 117),
    (56, 117),
    (44, 357),
    (46, 357),
    ];

pub fn play_melody() {
    init_midi_freq_table();

    for &(note, duration) in MELODY.iter() {
        if note == 0 {
            stop_beep();
        } else {
            beep(midi_note_to_freq(note) as u32);
        }
        wait_millis(duration);
    }
    stop_beep();
}
