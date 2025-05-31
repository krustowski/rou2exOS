const SLIP_END: u8 = 0xC0;
const SLIP_ESC: u8 = 0xDB;
const SLIP_ESC_END: u8 = 0xDC;
const SLIP_ESC_ESC: u8 = 0xDD;

/// Encodes a raw packet into a SLIP frame.
/// Returns how many bytes were written to `output`.
pub fn encode(input: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut out_pos = 0;

    // Start with SLIP_END
    if out_pos >= output.len() {
        return None;
    }

    if let Some(p) = output.get_mut(out_pos) {
        *p = SLIP_END;
    }

    out_pos += 1;

    for &b in input {
        match b {
            SLIP_END => {
                if out_pos + 2 > output.len() {
                    return None;
                }

                if let Some(p) = output.get_mut(out_pos) {
                    *p = SLIP_ESC;
                }
                if let Some(p) = output.get_mut(out_pos + 1) {
                    *p = SLIP_ESC_END;
                }

                out_pos += 2;
            }
            SLIP_ESC => {
                if out_pos + 2 > output.len() {
                    return None;
                }

                if let Some(p) = output.get_mut(out_pos) {
                    *p = SLIP_ESC;
                }
                if let Some(p) = output.get_mut(out_pos + 1) {
                    *p = SLIP_ESC_ESC;
                }

                out_pos += 2;
            }
            _ => {
                if out_pos >= output.len() {
                    return None;
                }

                if let Some(p) = output.get_mut(out_pos) {
                    *p = b
                }

                out_pos += 1;
            }
        }
    }

    // End with SLIP_END
    if out_pos >= output.len() {
        return None;
    }

    if let Some(p) = output.get_mut(out_pos) {
        *p = SLIP_END;
    }

    out_pos += 1;

    Some(out_pos)
}

/// Decodes a SLIP frame into a raw packet.
/// Returns how many bytes were written to `output`.
pub fn decode(input: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut out_pos = 0;
    let mut escape = false;

    for &b in input {
        match b {
            SLIP_END => {
                if out_pos > 0 {
                    return Some(out_pos);
                }
                // Ignore empty ENDs at start
            }
            SLIP_ESC => {
                escape = true;
            }
            _ => {
                if escape {
                    match b {
                        SLIP_ESC_END => output.get_mut(out_pos)?.clone_from(&SLIP_END),
                        SLIP_ESC_ESC => output.get_mut(out_pos)?.clone_from(&SLIP_ESC),
                        _ => return None, // Protocol error
                    }
                    escape = false;
                } else {
                    *output.get_mut(out_pos)? = b;
                }
                out_pos += 1;
            }
        }
    }

    None // Not finished
}

