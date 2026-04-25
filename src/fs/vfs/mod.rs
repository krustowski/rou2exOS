use spin::Mutex;

pub const MAX_MOUNTS: usize = 8;

#[derive(Clone, Copy, PartialEq)]
pub enum FsType {
    None,
    Root,
    Fat12, // floppy FAT12
}

#[derive(Clone, Copy)]
pub struct VfsMount {
    pub path: [u8; 32],
    pub path_len: usize,
    pub fs_type: FsType,
}

impl VfsMount {
    const fn empty() -> Self {
        Self {
            path: [0u8; 32],
            path_len: 0,
            fs_type: FsType::None,
        }
    }
}

pub struct VfsTable {
    mounts: [VfsMount; MAX_MOUNTS],
    count: usize,
}

impl VfsTable {
    const fn new() -> Self {
        Self {
            mounts: [VfsMount::empty(); MAX_MOUNTS],
            count: 0,
        }
    }

    pub fn mount(&mut self, path: &[u8], fs_type: FsType) -> bool {
        if self.count >= MAX_MOUNTS {
            return false;
        }

        let len = path.len().min(31);
        let mut m = VfsMount::empty();

        m.path[..len].copy_from_slice(&path[..len]);
        m.path_len = len;
        m.fs_type = fs_type;

        self.mounts[self.count] = m;
        self.count += 1;

        true
    }

    pub fn umount(&mut self, path: &[u8]) -> bool {
        for i in 0..self.count {
            if &self.mounts[i].path[..self.mounts[i].path_len] == path {
                for j in i..self.count - 1 {
                    self.mounts[j] = self.mounts[j + 1];
                }

                self.mounts[self.count - 1] = VfsMount::empty();
                self.count -= 1;

                return true;
            }
        }

        false
    }

    /// Returns the filesystem type and relative path for the longest-prefix matching mount.
    pub fn resolve<'a>(&self, path: &'a [u8]) -> Option<(FsType, &'a [u8])> {
        let mut best_len = 0usize;
        let mut best_type = FsType::None;

        for i in 0..self.count {
            let m = &self.mounts[i];

            if m.fs_type == FsType::None {
                continue;
            }

            let mp = &m.path[..m.path_len];

            if !path.starts_with(mp) {
                continue;
            }

            // Require exact match or path continues with '/'
            let tail_ok = path.len() == m.path_len || path.get(m.path_len) == Some(&b'/');

            if tail_ok && m.path_len >= best_len {
                best_len = m.path_len;
                best_type = m.fs_type;
            }
        }

        if best_type == FsType::None {
            return None;
        }

        let rel = path.get(best_len..).unwrap_or(b"");
        let rel = rel.strip_prefix(b"/").unwrap_or(rel);
        Some((best_type, rel))
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn get(&self, idx: usize) -> Option<&VfsMount> {
        if idx < self.count {
            Some(&self.mounts[idx])
        } else {
            None
        }
    }
}

pub static VFS: Mutex<VfsTable> = Mutex::new(VfsTable::new());

pub fn mount(path: &[u8], fs_type: FsType) -> bool {
    if let Some(mut vfs) = VFS.try_lock() {
        vfs.mount(path, fs_type)
    } else {
        false
    }
}

pub fn umount(path: &[u8]) -> bool {
    if let Some(mut vfs) = VFS.try_lock() {
        vfs.umount(path)
    } else {
        false
    }
}

/// If `path` is absolute and resolves to the Fat12 mount, returns the relative sub-path.
/// Callers use this to support both `/mnt/fat/FILE.EXT` and bare `FILE.EXT` inputs.
pub fn try_fat12_absolute<'a>(path: &'a [u8]) -> Option<&'a [u8]> {
    if let Some(vfs) = VFS.try_lock() {
        if let Some((FsType::Fat12, rel)) = vfs.resolve(path) {
            return Some(rel);
        }
    }
    None
}
