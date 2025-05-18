pub const USER: &[u8] = b"guest";
pub const HOST: &[u8] = b"rou2ex";

pub static mut PATH: &[u8] = b"/";
pub static mut PATH_CLUSTER: u16 = 19;

//
//
//

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

