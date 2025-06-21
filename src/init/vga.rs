use super::result::InitResult;
use crate::video::vga;

pub fn print_result() -> InitResult {
    vga::init_writer();

    InitResult::Passed
}
