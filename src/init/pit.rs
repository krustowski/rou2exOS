use crate::input::port::{read, write};

pub fn init_pit(frequency_hz: u32) {
    if frequency_hz == 0 {
        return;
    }

    //let divisor = 1_193_180 / frequency_hz;
    let divisor = 1_193_000_000 / frequency_hz;

    // PIT control port
    write(0x43, 0x36);
    write(0x40, (divisor & 0xFF) as u8);        // low byte
    write(0x40, ((divisor >> 8) & 0xFF) as u8); // high byte

    // Enable interrupts
    unsafe {
        core::arch::asm!("sti");
    }

}

pub unsafe fn remap_pic() {
    const PIC1: u16 = 0x20;
    const PIC2: u16 = 0xA0;
    const PIC1_COMMAND: u16 = PIC1;
    const PIC1_DATA: u16 = PIC1 + 1;
    const PIC2_COMMAND: u16 = PIC2;
    const PIC2_DATA: u16 = PIC2 + 1;

    const ICW1_INIT: u8 = 0x11;
    const ICW4_8086: u8 = 0x01;

    let offset1: u8 = 0x20; // remap IRQs 0-7 to IDT 32–39
    let offset2: u8 = 0x28; // remap IRQs 8-15 to IDT 40–47

    // Save current masks
    let a1 = read(PIC1_DATA);
    let a2 = read(PIC2_DATA);

    // Start initialization
    write(PIC1_COMMAND, ICW1_INIT);
    io_wait();
    write(PIC2_COMMAND, ICW1_INIT);
    io_wait();

    // Set vector offsets
    write(PIC1_DATA, offset1);
    io_wait();
    write(PIC2_DATA, offset2);
    io_wait();

    // Tell PIC1 there is a PIC2 at IRQ2 (0000 0100)
    write(PIC1_DATA, 4);
    io_wait();

    // Tell PIC2 its cascade identity (0000 0010)
    write(PIC2_DATA, 2);
    io_wait();

    // Set to 8086/88 mode
    write(PIC1_DATA, ICW4_8086);
    io_wait();
    write(PIC2_DATA, ICW4_8086);
    io_wait();

    // Restore saved masks
    write(PIC1_DATA, a1);
    write(PIC2_DATA, a2);
}


pub unsafe fn io_wait() {
    write(0x80, 0);
}

pub fn get_result() -> super::result::InitResult { //why??? why do this why not just ret value or something??
    debugln!("Remapping PIC");
    unsafe { remap_pic(); }

    debugln!("Starting 100Hz timer");
    init_pit(1); // 100Hz -> 10ms per tick???

    super::result::InitResult::Passed
}
