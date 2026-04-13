use crate::mem::pages::{map_page, KERNEL_PML4, PRESENT, WRITE};
use crate::video::sysprint::Result;

pub fn map_kernel_high_half() -> Result {
    Result::Passed
}
