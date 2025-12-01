use crate::init::boot::parse_multiboot2_info;
use crate::init::boot::FramebufferTag;
use crate::video::sysprint::Result;

pub fn parse_info(m2_ptr: u32, fb_tag: &mut FramebufferTag) -> Result {
    debug!("Multiboot2 pointer: ");
    debugn!(m2_ptr);
    debugln!("");

    unsafe {
        if parse_multiboot2_info(m2_ptr, fb_tag) > 0 {
            return Result::Passed;
        }
    }

    Result::Failed
}
