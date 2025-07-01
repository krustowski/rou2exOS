use crate::abi::idt::{install_isrs, load_idt};

pub fn get_result() -> super::result::InitResult {
    debugln!("Installing Exception handlers and ISRs");
    install_isrs();

    debugln!("Reloading IDT");
    load_idt();

    super::result::InitResult::Passed
}
