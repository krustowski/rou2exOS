use crate::api::idt::{init_int7f, load_idt};

pub fn get_result() -> super::result::InitResult {
    debugln!("Loading IDT");
    load_idt();

    debugln!("Installing int 0x7f ISR");
    init_int7f();

    super::result::InitResult::Passed
}
