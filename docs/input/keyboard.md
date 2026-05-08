# Keyboard

## IRQ1 Handler (`abi/idt.rs`)

On every keypress or key-release, the 8042 latches a scancode in port `0x60` and asserts IRQ1. `keyboard_handler` fires at IDT vector `0x21`:

1. Read scancode from port `0x60`.
2. Call `keyboard::push_scancode(sc)` — writes into `SCANCODE_BUF`.
3. Call `s.push_irq(sc)` for every active `RECEPTORS` subscriber.
4. Acknowledge master PIC: port `0x20` ← `0x20`.

The handler runs in interrupt context with interrupts disabled and no scheduler involvement.

---

## Scancode Buffer (`input/keyboard.rs`)

`SCANCODE_BUF: [u8; 2048]` is a kernel-internal circular buffer used exclusively by the shell loop.

```
SCANCODE_BUF: [u8; 2048]
SCANCODE_BUF_HEAD: usize     ← write index
SCANCODE_BUF_LOCKED: bool    ← true when no new scancode is available
```

**Write path** (`push_scancode`, called from IRQ1):
1. Write `scancode` to `SCANCODE_BUF[HEAD]`.
2. Increment and wrap `HEAD` modulo 2048.
3. Set `SCANCODE_BUF_LOCKED = false`.

**Read path** (`load_scancode`, called from shell):
1. Spin with `pause` until `SCANCODE_BUF_LOCKED == false`.
2. Set `SCANCODE_BUF_LOCKED = true`.
3. Return `SCANCODE_BUF[(HEAD - 1) % 2048]`.

This is a single-consumer, single-producer arrangement — only the shell reads from the buffer. There is no blocking sleep; the shell busy-waits using `pause` to yield to the CPU's pipeline without spinning on memory.

---

## Shell Loop (`keyboard_loop`)

`keyboard_loop()` is the kernel shell. It runs as a dedicated task (slot 0 of `KSTACK_POOL`) and never returns.

```
loop:
    key ← load_scancode()

    if key & 0x80 != 0:          // key-release event
        scancode_to_ascii(key)   // update modifier state (Shift, CapsLock)
        continue

    match key:
        0x0E (Backspace)  → erase last char, decrement input_len
        0x0F (Tab)        → tab completion (see below)
        0x1C (Enter)      → submit line to cmd::handle(), clear buffer, print prompt
        0x1D (Ctrl)       → set ctrl_down = true
        0x26 + ctrl_down  → Ctrl+L: clear screen, reset buffer, print prompt
        _                 → scancode_to_ascii(key) → append to input_buffer if printable
```

Input buffer: `[u8; 128]`. Once full (128 bytes), further printable characters are silently dropped.

---

## Scancode Set 1 Translation (`scancode_to_ascii`)

Translates PS/2 Set 1 make-codes to ASCII bytes. Returns `None` for non-printable keys or modifier-only events.

Modifier state is tracked in two `static mut` booleans:

| Variable | Updated on scancode |
|----------|-------------------|
| `SHIFT_PRESSED` | `0x2A`/`0x36` (make), `0xAA`/`0xB6` (break) |
| `CAPS_LOCK_ON` | `0x3A` (make, toggled) |

Letter key rule: `upper` is emitted when `CAPS_LOCK_ON XOR SHIFT_PRESSED` is true.

### Key Map (selected)

| Scancode | Normal | Shifted |
|----------|--------|---------|
| `0x02–0x0B` | `1–9, 0` | `!, @, #, $, %, ^, &, *, (, )` |
| `0x0C` | `-` | `_` |
| `0x0D` | `=` | `+` |
| `0x0E` | Backspace (8) | — |
| `0x0F` | Tab | — |
| `0x10–0x19` | `q w e r t y u i o p` | uppercase |
| `0x1A` | `[` | `{` |
| `0x1B` | `]` | `}` |
| `0x1C` | Enter (`\n`) | — |
| `0x1D` | Ctrl | — |
| `0x1E–0x26` | `a s d f g h j k l` | uppercase |
| `0x27` | `;` | `:` |
| `0x28` | `'` | `"` |
| `0x29` | `` ` `` | `~` |
| `0x2A` | Shift (L) | — |
| `0x2B` | `\` | `\|` |
| `0x2C–0x32` | `z x c v b n m` | uppercase |
| `0x33` | `,` | `<` |
| `0x34` | `.` | `>` |
| `0x35` | `/` | `?` |
| `0x36` | Shift (R) | — |
| `0x39` | Space | — |
| `0x3A` | Caps Lock (toggle) | — |

Break-codes (bit 7 set) for the above are `scancode | 0x80`. All unrecognized scancodes return `None`.

---

## Tab Completion

When `Tab` (scancode `0x0F`) is pressed:

1. Split the current input buffer at the first space: `(cmd, prefix)`.
2. If `prefix` is empty, run `cmd::handle(b"help")` and return.
3. Scan FAT12 directory entries in the current working directory.
4. Pad `prefix` to 11 characters (FAT 8.3 name format: `pad_prefix()`).
5. On a match:
   - Backspace over `prefix.len()` characters.
   - Print the full matched name in magenta.
   - Replace the prefix in `input_buffer` with the completed name.
   - For `cd`: skip file entries (only directories match).

Tab completion operates on FAT12 only (no ISO9660 or VFS dispatch).

---

## Keyboard Pipe — Userland Subscriber Fan-out (`input/irq.rs`)

When the shell is not the consumer, userland programs can receive raw scancodes directly via a subscriber mechanism.

### Subscriber Table

```rust
pub static mut RECEPTORS: [Subscriber; 5]
```

Each `Subscriber` has:

| Field | Type | Description |
|-------|------|-------------|
| `buf_ptr` | `u64` | Virtual address of the userland mailbox byte (or 0 if unused) |
| `pid` | `usize` | PID of the subscribed process (0 = free slot) |
| `kbuf` | `[u8; 256]` | Kernel-side ring buffer |
| `head`/`tail` | `AtomicUsize` | Ring buffer write/read indices (SeqCst) |

### Two Read Paths

**Mailbox path** (polling-friendly): `push_irq` writes to `*buf_ptr` only when the current value is `0`. Userland polls this byte; after reading it must clear it to `0` to receive the next scancode.

**Ring buffer path** (drain-friendly): `push_irq` always enqueues into `kbuf`, regardless of the mailbox state. Userland calls syscall `0x03/0x03` which invokes `copy_to_user()` to drain up to 16 bytes from the ring.

Both paths are written on every IRQ1 event for every active subscriber.

### Syscall Interface

| Syscall | Sub-opcode | Action |
|---------|-----------|--------|
| `0x03` | `0x01` | `pipe_subscribe(addr)` — register calling process, set mailbox address |
| `0x03` | `0x02` | `pipe_unsubscribe(_addr)` — deregister calling process |
| `0x03` | `0x03` | `copy_to_user(arg2, 16)` — drain up to 16 bytes from ring buffer |

`pipe_subscribe` finds the first slot where `pid == 0`, sets `pid` and `buf_ptr`, and clears the ring buffer. `pipe_unsubscribe` clears the slot by PID. Maximum 5 simultaneous subscribers.
