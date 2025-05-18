extern "C" {
    pub static multiboot_ptr: u64;
}

extern "C" {
    static debug_flag: u8;
}

pub fn debug_enabled() -> bool {
    unsafe { 
        debug_flag != 0 
    }
}

