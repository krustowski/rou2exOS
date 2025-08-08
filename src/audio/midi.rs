use super::beep::{beep, stop_beep};
use crate::fs::fat12;

#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    pub note: u8,      
    pub duration: u16, 
}

pub struct MidiFile<'a> {
    pub division: u16,
    pub events: [MidiEvent; 256],
    pub event_count: usize,
    pub raw: &'a [u8],
}

pub fn wait_millis(ms: u16) {
    for _ in 0..(ms as u32 * 75_000) {
        unsafe { core::arch::asm!("nop") };
    }
}

pub static MIDI_FREQ_TABLE: [u16; 128] = [
    8, 9, 9, 10, 10, 11, 12, 13, 14, 15, 16, 17, // 0–11
    18, 19, 21, 22, 23, 25, 26, 28, 29, 31, 33, 35, // 12–23
    37, 39, 41, 44, 46, 49, 52, 55, 58, 62, 65, 69, // 24–35
    73, 78, 82, 87, 92, 98, 104, 110, 117, 123, 131, 139, // 36–47
    147, 156, 165, 175, 185, 196, 208, 220, 233, 247, 262, 277, // 48–59
    294, 311, 330, 349, 370, 392, 415, 440, 466, 494, 523, 554, // 60–71
    587, 622, 659, 698, 740, 784, 831, 880, 932, 988, 1047, 1109, // 72–83
    1175, 1245, 1319, 1397, 1480, 1568, 1661, 1760, 1865, 1976, 2093, 2217, // 84–95
    2349, 2489, 2637, 2794, 2960, 3136, 3322, 3520, 3729, 3951, 4186, 4435, // 96–107
    4699, 4978, 5274, 5588, 5920, 6272, 6645, 7040, 7458, 7902, 8372, 8869, // 108–119
    9397, 9956, 10548, 11175, 11840, 12544, 13290, 14080 // 120–127
];

pub fn midi_note_to_freq(note: u8) -> u16 {
    unsafe {
        return MIDI_FREQ_TABLE[note as usize];
    }
}

pub static TEST_MELODY: &[(u8, u16)] = &[
    (69, 200), // A4 - 440 Hz
    (0, 50),
    (71, 200), // B4
    (0, 50),
    (72, 200), // C5
    (0, 50),
    (74, 200), // D5
    (0, 50),
    (76, 400), // E5
];

pub fn play_melody() {
    for &(note, duration) in TEST_MELODY.iter() {
        stop_beep();

        if note != 0 {
            let freq = midi_note_to_freq(note);
            beep(freq as u32);
        }

        wait_millis(duration);
    }

    stop_beep();
}

fn read_varlen(data: &[u8]) -> (u32, usize) {
    let mut result = 0;
    let mut i = 0;
    loop {
        let byte = data[i];
        result = (result << 7) | (byte & 0x7F) as u32;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
    }
    (result, i)
}

pub fn parse_midi_format0(data: &[u8]) -> Option<MidiFile> {
    if &data[0..4] != b"MThd" { return None; }

    let header_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if header_len != 6 { return None; }

    let format = u16::from_be_bytes([data[8], data[9]]);
    if format != 0 { return None; }

    let num_tracks = u16::from_be_bytes([data[10], data[11]]);
    if num_tracks != 1 { return None; }

    let division = u16::from_be_bytes([data[12], data[13]]);
    let mut pos = 14;

    if &data[pos..pos + 4] != b"MTrk" { return None; }
    let track_len = u32::from_be_bytes([data[pos+4], data[pos+5], data[pos+6], data[pos+7]]) as usize;
    pos += 8;

    let track_end = pos + track_len;
    let mut time = 0u32;
    let mut last_status = 0u8;
    let mut output = [MidiEvent { note: 0, duration: 0 }; 256];
    let mut out_idx = 0;

    while pos < track_end && out_idx < output.len() {
        if pos >= data.len() {
            break;
        }

        // Delta time
        let (delta, delta_len) = read_varlen(&data[pos..]);
        pos += delta_len;
        time += delta;

        // Event type
        let mut status = data[pos];
        if status < 0x80 {
            status = last_status;
        } else {
            pos += 1;
            last_status = status;
        }

        if status & 0xF0 == 0x90 {
            // Note On
            let note = data[pos];
            let velocity = data[pos + 1];
            pos += 2;

            if velocity > 0 {
                let duration_ticks = 240;
                let ms = ticks_to_ms(duration_ticks, division);
                output[out_idx] = MidiEvent { note, duration: ms };
                out_idx += 1;
            }
        } else if status & 0xF0 == 0x80 {
            // Note Off — ignore
            pos += 2;
        } else if status == 0xFF {
            // Meta event
            let meta_type = data[pos];
            let (len, len_len) = read_varlen(&data[pos + 1..]);
            pos += 1 + len_len + len as usize;
        } else {
            pos += 2;
        }
    }

    Some(MidiFile {
        division,
        events: output,
        event_count: out_idx,
        raw: data,
    })
}

fn ticks_to_ms(ticks: u32, division: u16) -> u16 {
    let micros = (ticks as u64 * 500_000) / division as u64;
    (micros / 1000) as u16
}

pub fn play_midi(midi: &MidiFile) {
    for i in 0..midi.event_count {
        let e = midi.events[i];
        super::beep::stop_beep();
        if e.note > 0 {
            let freq = super::midi::midi_note_to_freq(e.note);
            super::beep::beep(freq as u32);
        }
        super::midi::wait_millis(e.duration);
    }
    super::beep::stop_beep();
}

pub fn play_midi_file() {
    if let Some(data) = read_file() {
        if let Some(midi) = parse_midi_format0(data) {
            play_midi(&midi);
        } else {
            println!("Invalid MIDI file");
        }
    } else {
        println!("Could not read file");
    }
}

static mut BUF: [u8; 4096] = [0u8; 4096];

fn read_file() -> Option<&'static [u8]> {
    let floppy = fat12::block::Floppy::init();

    match fat12::fs::Filesystem::new(&floppy) {
        Ok(fs) => {
            fs.for_each_entry(0, | entry | {
                if entry.ext.starts_with(b"MID") {
                    unsafe {
                        fs.read_file(entry.start_cluster, &mut BUF);
                    }
                }
            });
        }
        Err(_) => {}
    }

    unsafe {
        Some(&BUF)
    }
}

