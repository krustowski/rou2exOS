use crate::video::sysprint::{Result};
use crate::init::boot::{FramebufferTag};
use crate::init::boot::{parse_multiboot2_info};

pub fn parse_info(multiboot_ptr: u64, fb_tag: &FramebufferTag) -> Result {
    unsafe {
        debug!("Multiboot2 pointer: ");
        debugn!(multiboot_ptr);
        debugln!("");

        if parse_multiboot2_info((multiboot_ptr as u32) as usize, fb_tag) > 0 {
            return Result::Passed;
        }
    }

    debug!("Multiboot2 pointer: ");
    debugn!(multiboot_ptr);
    debugln!("");

    Result::Failed
}
