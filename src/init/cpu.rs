use core::arch::asm;
use crate::video::sysprint::{Result};

pub fn check() -> Result {
    let mode = check_cpu_mode();

    enable_sse();
    enable_syscalls();

    if mode.len() > 5 && mode.as_bytes()[0..4] == *b"Long" {
        return Result::Passed;
    }

    Result::Failed
}

fn enable_sse() {
    unsafe {
        // Enable SSE + FXSR in CR4
        let mut cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4);
        cr4 |= (1 << 9) | (1 << 10); // OSFXSR (bit 9), OSXMMEXCPT (bit 10)
        asm!("mov cr4, {}", in(reg) cr4);

        // Clear EM (bit 2), Set MP (bit 1) in CR0
        let mut cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0);
        cr0 &= !(1 << 2); // Clear EM (disable emulation)
        cr0 |= 1 << 1;    // Set MP (monitor co-processor)
        asm!("mov cr0, {}", in(reg) cr0);
    }
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
            // Store eax into result
            inout("eax") eax => result,    
            out("ecx") _,                  
            out("edx") _,                  
        );
    }
    result
}

//
//  CPU SYSCALL
//

const IA32_EFER: u32 = 0xC0000080;
const IA32_LSTAR: u32 = 0xC0000082;

unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags)
    );
}

unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

// Your syscall handler (just returns for now)
#[unsafe(naked)]
unsafe extern "C" fn syscall_handler() {
    core::arch::naked_asm!(
        "swapgs",           // swap GS base to kernel GS base
        "push rcx",         // save RCX (return RIP)
        "push r11",         // save R11 (RFLAGS)
        // here you could call Rust code or handle syscall number in rax
        "pop r11",
        "pop rcx",
        "sysretq",
    );
}

fn enable_syscalls() {
    unsafe {
        // Set IA32_LSTAR to syscall_handler address
        #[expect(clippy::fn_to_numeric_cast)]
        wrmsr(IA32_LSTAR, syscall_handler as u64);

        // Enable syscall/sysret in EFER (bit 0 = SCE)
        let mut efer = rdmsr(IA32_EFER);
        efer |= 1;  // set SCE bit
        wrmsr(IA32_EFER, efer);
    }
}

