pub fn shutdown() {
    unsafe {
        // ACPI shutdown port (common for Bochs/QEMU/VirtualBox)
        const SLP_TYPA: u16 = 0x2000;
        const SLP_EN: u16 = 1 << 13;

        // Fallback PM1a control port address
        const PM1A_CNT_PORT: u16 = 0x604;

        // Write shutdown command
        let value = SLP_TYPA | SLP_EN;

        core::arch::asm!(
            "out dx, ax",
            in("dx") PM1A_CNT_PORT,
            in("ax") value,
        );
    }

    // Freeze in case of the shutdown failure (no ACPI)
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
