from mido import MidiFile

DEFAULT_TEMPO = 500000  # microseconds per beat (120 BPM)

def midi_to_rust_array(path):
    mid = MidiFile(path)
    ticks_per_beat = mid.ticks_per_beat or 480
    tempo = DEFAULT_TEMPO
    events = []

    for track in mid.tracks:
        time = 0
        for i, msg in enumerate(track):
            time += msg.time
            if msg.type == 'set_tempo':
                tempo = msg.tempo
            if msg.type == 'note_on' and msg.velocity > 0:
                note = msg.note
                dur_ticks = 0
                for m in track[i+1:]:
                    dur_ticks += m.time
                    if (m.type == 'note_off' and m.note == note) or \
                       (m.type == 'note_on' and m.note == note and m.velocity == 0):
                        break
                duration_ms = int(dur_ticks * (tempo / 1000) / ticks_per_beat)
                events.append((note, duration_ms))

    print("pub static MELODY: &[(u8, u16)] = &[")
    for note, dur in events:
        print(f"    ({note}, {dur}),")
    print("];")

# Replace with your local file path
midi_to_rust_array("egg.midi")

