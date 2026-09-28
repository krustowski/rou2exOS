# HD Audio

`src/audio/hda.rs` drives an Intel High Definition Audio controller: one output stream of 16-bit stereo PCM, which programs fill through [syscall `0x3f`](../abi/syscalls/video_audio.md#0x3f-hd-audio-pcm). It is what Memento's Video window plays a film's sound through. The PC speaker (`0x1a`, `0x1b`) is separate and unchanged.

## Bringing it up

Nothing happens at boot. The first `0x3f` call, or the shell's `hda` command, does this once:

1. **Find the controller** by what it is, PCI class `04` subclass `03`, whoever made it. Turn on memory decoding and bus mastering. Registers are read through BAR0, which has to lie below 4 GiB, inside the identity map. On Intel controllers, set traffic class 0 (`TCSEL`, config `0x44`) and clear `DEVC` no-snoop (config `0x78`, bit 11), as Linux does, so the controller sees the kernel's cached ring.
2. **Reset** (`GCTL.CRST` down and up), then wait for the codecs to announce themselves in `STATESTS`. The first codec present is used.
3. **CORB and RIRB**, the command and response rings, at their largest size, polled. The controller stops taking commands once `RINTCNT` responses have arrived, until the response interrupt flag is cleared. With no interrupt handler, the driver clears `RIRBSTS` around every command itself. A controller whose rings do not answer at all is driven through the immediate command registers (`ICOI`/`ICII`/`ICIS`) instead.
4. **The codec.** Find the audio function group and power it to D0. For every pin that can drive an output (pin capability bit 4), is connected to something (configuration default, connectivity not "none"), and is a line out, a speaker or headphones:
   - search its connection list, through mixers and selectors (up to five deep), for a DAC;
   - power everything on the path, select the connection taken at pins and selectors, and unmute the amplifiers on the path at 0 dB (the offset step the amplifier reports);
   - turn the pin's output on (and the headphone amplifier for headphones), and the external amplifier (EAPD) where the pin has one.

   Every DAC reached is tuned to stream 1, so a board with speakers and a headphone jack plays on both.

## Playing

The stream is the first output stream descriptor (after the input ones). It plays a ring of **128 KiB** in the kernel's `.bss` (about 0.68 s at 48 kHz), described by a 4-entry buffer descriptor list, round and round. Opening sets the format (16-bit, 2 channels, and the rate as a base of 48 or 44.1 kHz with a multiplier and divisor), clears the ring and starts the DMA.

There are no interrupts. The timer tick and every `0x3f` call read the stream's position register (`LPIB`) with interrupts off. **What has just been played is zeroed at once.** So a program that stops writing, or dies, leaves silence going round the ring, not the last bit of sound over and over. A write takes as many whole samples as there is room for, never waits, and keeps 1 KiB clear of the play position.

Pause stops the DMA engine and keeps the ring and the position; resume starts it again.

## Limits

- One stream, one program at a time: the last to open takes it over.
- 16-bit stereo only, at 8, 11.025, 16, 22.05, 24, 32, 44.1, 48, 88.2 or 96 kHz, as the controller converts them. No mixing and no resampling.
- The first codec with an audio function group; HDMI/DisplayPort codecs (pins that are neither line out, speaker nor headphones) are left alone.
- No volume control: the path is set at 0 dB. No jack sensing: every connected output pin plays.

## Testing it

The `hda` shell command (text kernel) says what was found: the controller's PCI address and ids, command mode, codec, function group, DACs and pins. `hda tone` plays a second of 440 Hz from the kernel itself, which is the first thing to try on new hardware. If the command reports `No sound: ...`, the reason and the codec's first answers (`probe`) are printed.

In QEMU, record what is played:

```shell
qemu-system-x86_64 ... -device intel-hda -device hda-output,audiodev=snd0 \
    -audiodev wav,id=snd0,path=out.wav
```

`hda tone` there gives a second of 440 Hz at a quarter of full scale in `out.wav`, and a film with MP2 audio in Memento's Video window gives its sound.
