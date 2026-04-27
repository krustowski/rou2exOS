use spin::Mutex;

pub static SYSTEM_CONFIG: Mutex<SystemConfig> = Mutex::new(SystemConfig::new());

pub struct SystemConfig {
    pub user: [u8; 32],
    pub host: [u8; 32],
    pub path: [u8; 32],
    pub path_len: usize,
    pub path_cluster: u16,
    pub version: [u8; 16],
}

impl SystemConfig {
    const fn new() -> Self {
        Self {
            user: *b"root                            ",
            host: *b"rourex                          ",
            path: *b"/                               ",
            path_len: 1,
            path_cluster: 0,
            version: *b"v0.11.3         ",
        }
    }

    pub fn set_user(&mut self, new_user: &[u8]) {
        let len = new_user.len().min(32);
        self.user[..len].copy_from_slice(&new_user[..len]);
        self.user[len..].fill(b' ');
    }

    pub fn get_user(&self) -> &[u8] {
        let end = self
            .user
            .iter()
            .rposition(|&b| b != b' ')
            .map_or(0, |i| i + 1);
        &self.user[..end]
    }

    pub fn set_host(&mut self, new_host: &[u8]) {
        let len = new_host.len().min(32);
        self.host[..len].copy_from_slice(&new_host[..len]);
        self.host[len..].fill(b' ');
    }

    pub fn get_host(&self) -> &[u8] {
        let end = self
            .host
            .iter()
            .rposition(|&b| b != b' ')
            .map_or(0, |i| i + 1);
        &self.host[..end]
    }

    pub fn set_path(&mut self, new_path: &[u8], cluster: u16) {
        let len = new_path.len().min(32);
        self.path[..len].copy_from_slice(&new_path[..len]);
        self.path[len..].fill(b' ');
        self.path_len = len;
        self.path_cluster = cluster;
    }

    pub fn get_path(&self) -> &[u8] {
        let s = &self.path[..self.path_len.min(32)];
        let end = s.iter().rposition(|&b| b != b' ').map_or(0, |i| i + 1);
        &s[..end]
    }

    pub fn get_path_cluster(&self) -> u16 {
        self.path_cluster
    }

    pub fn get_version(&self) -> &[u8] {
        let end = self
            .version
            .iter()
            .rposition(|&b| b != b' ')
            .map_or(0, |i| i + 1);
        &self.version[..end]
    }
}

static mut PROMPT_BUF: [u8; 80] = [0u8; 80];

/// Assembles `user@host:path > ` into a static buffer and returns a slice of it.
pub fn get_prompt() -> &'static [u8] {
    unsafe {
        let mut pos = 0usize;

        let buf = &mut PROMPT_BUF;
        let mut push = |s: &[u8]| {
            for &b in s {
                if pos < buf.len() {
                    buf[pos] = b;
                    pos += 1;
                }
            }
        };

        if let Some(cfg) = SYSTEM_CONFIG.try_lock() {
            push(cfg.get_user());
            push(b"@");
            push(cfg.get_host());
            push(b":");
            push(cfg.get_path());
            push(b" > ");
        } else {
            push(b"$ ");
        }

        core::slice::from_raw_parts(PROMPT_BUF.as_ptr(), pos)
    }
}

//
//
//

extern "C" {
    pub static mut p4_table: [u64; 512];
}

extern "C" {
    pub static p3_table: u64;
}

extern "C" {
    pub static p2_table: u64;
}

extern "C" {
    pub static mut p3_fb_table: [u64; 512];
    pub static mut p2_fb_table: [u64; 512];
    pub static mut p1_fb_table: [u64; 512];
    pub static mut p1_fb_table_2: [u64; 512];
}

extern "C" {
    pub static multiboot_ptr: u32;
}

extern "C" {
    static debug_flag: u8;
}

pub fn debug_enabled() -> bool {
    unsafe { debug_flag != 0 }
}

extern "C" {
    static __stack_start: u8;
    static __stack_end: u8;
}
