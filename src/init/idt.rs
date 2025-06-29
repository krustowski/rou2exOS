use crate::api::idt::{init_int80, load_idt};

pub fn get_result() -> super::result::InitResult {
    debugln!("Loading IDT");
    load_idt();

    debugln!("Installing int 0x80 ISR");
    init_int80();

    super::result::InitResult::Passed
}
