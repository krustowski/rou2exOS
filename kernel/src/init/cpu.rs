use core::arch::asm;
use crate::vga;

pub fn print_mode(vga_index: &mut isize) {
    vga::write::string(vga_index, b"CPU mode: ", 0x0f);
    vga::write::string(vga_index, check_cpu_mode().as_bytes(), 0x0f);

    vga::write::newline(vga_index);
    vga::write::newline(vga_index);
}

/// Function to check CPU mode using CPUID instruction
fn check_cpu_mode() -> &'static str {
    let cpuid_supported = cpuid(0x1);

    if cpuid_supported == 0 {
        return "Real Mode (CPUID not supported)";
    }

    let cpuid_value = cpuid(0x80000000);

    // Check for 64-bit long mode (if CPUID supports extended functions)
    if cpuid_value >= 0x80000001 {
        return "Long Mode (64-bit mode)";
    }

    // Otherwise, it is protected mode
    "Protected Mode (32-bit)"
}

/// Inline assembly function to execute CPUID
fn cpuid(eax: u32) -> u32 {
    let result: u32;
    unsafe {
        asm!(
            "cpuid",
            inout("eax") eax => result,    // Use eax for input and output (stored in `result`)
            out("ecx") _,                  // Don't use `ecx`, just discard it
            out("edx") _,                  // Don't use `edx`, just discard it
        );
    }
    result
}
