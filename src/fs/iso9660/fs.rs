use super::block::{Atapi, BLOCK_SIZE};

#[derive(Copy, Clone)]
pub struct IsoEntry {
    pub name: [u8; 32],
    pub name_len: u8,
    pub is_dir: bool,
    pub lba: u32,
    pub size: u32,
}

impl Default for IsoEntry {
    fn default() -> Self {
        Self { name: [0u8; 32], name_len: 0, is_dir: false, lba: 0, size: 0 }
    }
}

pub struct Iso9660 {
    pub atapi: Atapi,
    pub root_lba: u32,
    pub root_size: u32,
}

impl Iso9660 {
    pub fn probe() -> Option<Self> {
        let atapi = Atapi::new();
        let mut buf = [0u8; BLOCK_SIZE];
        if !atapi.read_block(16, &mut buf) {
            return None;
        }
        // PVD magic: type=1, identifier="CD001"
        if buf[0] != 0x01 || &buf[1..6] != b"CD001" {
            return None;
        }
        // Root directory record embedded at PVD offset 156
        let root_lba  = u32::from_le_bytes([buf[158], buf[159], buf[160], buf[161]]);
        let root_size = u32::from_le_bytes([buf[166], buf[167], buf[168], buf[169]]);
        Some(Self { atapi, root_lba, root_size })
    }

    pub fn list_dir(&self, lba: u32, size: u32, out: &mut [IsoEntry; 64]) -> usize {
        let mut count = 0usize;
        let mut block_lba = lba;
        let mut remaining = size as usize;

        'outer: while remaining > 0 {
            let mut buf = [0u8; BLOCK_SIZE];
            if !self.atapi.read_block(block_lba, &mut buf) { break; }

            let chunk = BLOCK_SIZE.min(remaining);
            let mut off = 0usize;
            while off < chunk {
                let rec_len = buf[off] as usize;
                if rec_len < 33 { break; } // 0 = sector padding; <33 = too small to be valid
                if off + rec_len > BLOCK_SIZE { break; }

                let name_len = buf[off + 32] as usize;
                // Skip . (0x00) and .. (0x01) special entries
                if name_len == 1 && (buf[off + 33] == 0 || buf[off + 33] == 1) {
                    off += rec_len;
                    continue;
                }

                if count >= 64 { break 'outer; }

                let entry_lba  = u32::from_le_bytes([buf[off+2], buf[off+3], buf[off+4], buf[off+5]]);
                let entry_size = u32::from_le_bytes([buf[off+10], buf[off+11], buf[off+12], buf[off+13]]);
                let is_dir     = buf[off + 25] & 0x02 != 0;

                if off + 33 + name_len > BLOCK_SIZE { break; }
                let raw_name  = &buf[off + 33 .. off + 33 + name_len];
                let stripped  = strip_version(raw_name);
                let copy_len  = stripped.len().min(32);

                let mut e = IsoEntry::default();
                for (i, &b) in stripped[..copy_len].iter().enumerate() { e.name[i] = to_lower(b); }
                e.name_len = copy_len as u8;
                e.is_dir   = is_dir;
                e.lba      = entry_lba;
                e.size     = entry_size;

                out[count] = e;
                count += 1;
                off += rec_len;
            }

            block_lba += 1;
            remaining = remaining.saturating_sub(BLOCK_SIZE);
        }
        count
    }

    pub fn find(&self, dir_lba: u32, dir_size: u32, name: &[u8]) -> Option<IsoEntry> {
        let mut block_lba = dir_lba;
        let mut remaining = dir_size as usize;

        while remaining > 0 {
            let mut buf = [0u8; BLOCK_SIZE];
            if !self.atapi.read_block(block_lba, &mut buf) { break; }

            let chunk = BLOCK_SIZE.min(remaining);
            let mut off = 0usize;
            while off < chunk {
                let rec_len = buf[off] as usize;
                if rec_len < 33 { break; }
                if off + rec_len > BLOCK_SIZE { break; }

                let name_len = buf[off + 32] as usize;
                if name_len == 1 && (buf[off + 33] == 0 || buf[off + 33] == 1) {
                    off += rec_len;
                    continue;
                }

                if off + 33 + name_len > BLOCK_SIZE { off += rec_len; continue; }
                let raw_name = &buf[off + 33 .. off + 33 + name_len];
                let stripped = strip_version(raw_name);

                if names_match(stripped, name) {
                    let entry_lba  = u32::from_le_bytes([buf[off+2], buf[off+3], buf[off+4], buf[off+5]]);
                    let entry_size = u32::from_le_bytes([buf[off+10], buf[off+11], buf[off+12], buf[off+13]]);
                    let is_dir     = buf[off + 25] & 0x02 != 0;
                    let copy_len   = stripped.len().min(32);

                    let mut e = IsoEntry::default();
                    for (i, &b) in stripped[..copy_len].iter().enumerate() { e.name[i] = to_lower(b); }
                    e.name_len = copy_len as u8;
                    e.is_dir   = is_dir;
                    e.lba      = entry_lba;
                    e.size     = entry_size;
                    return Some(e);
                }
                off += rec_len;
            }

            block_lba += 1;
            remaining = remaining.saturating_sub(BLOCK_SIZE);
        }
        None
    }

    /// Resolve a slash-separated path relative to the ISO root.
    /// Returns the terminal IsoEntry, or None if any component is not found.
    pub fn resolve(&self, path: &[u8]) -> Option<IsoEntry> {
        let path = path.strip_prefix(b"/").unwrap_or(path);
        if path.is_empty() {
            return Some(IsoEntry { is_dir: true, lba: self.root_lba, size: self.root_size, ..IsoEntry::default() });
        }

        let mut cur_lba  = self.root_lba;
        let mut cur_size = self.root_size;
        let mut remaining = path;
        let mut last = None;

        while !remaining.is_empty() {
            let (component, rest) = match remaining.iter().position(|&b| b == b'/') {
                Some(p) => (&remaining[..p], remaining.get(p + 1..).unwrap_or(&[])),
                None    => (remaining, &[][..]),
            };
            if component.is_empty() {
                remaining = rest;
                continue;
            }

            let entry = self.find(cur_lba, cur_size, component)?;
            if !rest.is_empty() {
                if !entry.is_dir { return None; }
                cur_lba  = entry.lba;
                cur_size = entry.size;
            }
            last = Some(entry);
            remaining = rest;
        }
        last
    }

    pub fn read_file(&self, entry: &IsoEntry, buf: &mut [u8]) -> usize {
        let total = (entry.size as usize).min(buf.len());
        let mut done = 0usize;
        let mut block_lba = entry.lba;

        while done < total {
            let mut block = [0u8; BLOCK_SIZE];
            if !self.atapi.read_block(block_lba, &mut block) { break; }
            let to_copy = (total - done).min(BLOCK_SIZE);
            buf[done .. done + to_copy].copy_from_slice(&block[..to_copy]);
            done += to_copy;
            block_lba += 1;
        }
        done
    }
}

fn strip_version(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|&b| b == b';') {
        Some(pos) => &name[..pos],
        None      => name,
    }
}

fn to_lower(b: u8) -> u8 { if b.is_ascii_uppercase() { b + 32 } else { b } }
fn to_upper(b: u8) -> u8 { if b.is_ascii_lowercase() { b - 32 } else { b } }

fn names_match(iso_name: &[u8], query: &[u8]) -> bool {
    if iso_name.len() != query.len() { return false; }
    iso_name.iter().zip(query.iter()).all(|(&a, &b)| to_upper(a) == to_upper(b))
}
