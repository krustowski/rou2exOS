use core::arch::asm;
use x86_64::structures::idt::{PageFaultErrorCode};

use crate::vga;

//
// GDT
//

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct GdtDescriptor {
    limit: u16,
    base: u64,
}

#[repr(C)]
struct Gdt([u64; 3]);

// Proper 64-bit code and data segment descriptors
static GDT: Gdt = Gdt([
    0,
    0x00cf9a000000ffff, // 64-bit code (L-bit set)
    0x00cf92000000ffff, // data segment
]);

fn get_gdtr() -> GdtDescriptor {
    GdtDescriptor {
        limit: (core::mem::size_of::<Gdt>() - 1) as u16,
        base: unsafe { &GDT as *const _ as u64 },
    }
}

pub fn load_gdt() {
    let gdtr = get_gdtr();

    let cr0_flags: u64 = (1 << 0) | (1 << 31); // PE | PG
    unsafe {
        asm!(
            "cli",
            "lgdt [{}]",
            in(reg) cr0_flags,
            options(nostack, preserves_flags)
        );
    }
}

//
// Paging
//

#[repr(align(4096))]
pub struct PageTable([u64; 512]);

static mut PML4: PageTable = PageTable([0; 512]);
static mut PDPT: PageTable = PageTable([0; 512]);
static mut PD: PageTable = PageTable([0; 512]);

pub unsafe fn setup_paging() -> u64 {
    // Map first 1 GiB (1 PD entry)
    PD.0[0] = 0x0000_0000_0000_0000 | 0b10000011; // 2MB page, Present + RW + PS

    // Link PD to PDPT
    PDPT.0[0] = &PD.0 as *const _ as u64 | 0b11;

    // Link PDPT to PML4
    PML4.0[0] = &PDPT.0 as *const _ as u64 | 0b11;

    // Return physical address of PML4 — assuming identity mapping
    &PML4.0 as *const _ as u64
}


const EFER_MSR: u32 = 0xC000_0080;
const EFER_LME: u64 = 1 << 8;
const CR0_PE: u64 = 1 << 0;
const CR0_PG: u64 = 1 << 31;
const CR4_PAE: u64 = 1 << 5;

pub unsafe fn enter_long_mode(pml4_phys_addr: u64) {
    // Enable PAE (CR4)
    asm!(
        "mov rax, cr4",
        "or rax, {pae}",
        "mov cr4, rax",
        pae = const CR4_PAE,
        options(nostack, preserves_flags),
    );

    // Enable LME (Long Mode Enable) in EFER
    let mut edx: u32;
    let mut eax: u32;
    asm!(
        "rdmsr",
        in("ecx") EFER_MSR,
        out("edx") edx,
        out("eax") eax,
    );
    let mut efer_val = ((edx as u64) << 32) | (eax as u64);
    efer_val |= EFER_LME;
    let edx = (efer_val >> 32) as u32;
    let eax = (efer_val & 0xFFFF_FFFF) as u32;
    asm!(
        "wrmsr",
        in("ecx") EFER_MSR,
        in("edx") edx,
        in("eax") eax,
    );

    // Load CR3 with PML4 physical address
    asm!(
        "mov cr3, {0}",
        in(reg) pml4_phys_addr,
        options(nostack, preserves_flags),
    );

    // Enable PG and PE in CR0
    asm!(
        "mov rax, cr0",
        "or rax, {}",
        "mov cr0, rax",
        in(reg) (CR0_PE | CR0_PG),
        options(nostack, preserves_flags),
    );
}



/*
            // Enable PAE (bit 5) in cr4
            "mov rax, cr4",
            "or rax, 1 << 5",
            "mov cr4, rax",

            // Set LME (Long Mode Enable) in EFER MSR
            "mov ecx, 0xC0000080",
            "rdmsr",
            "or eax, 1 << 8",
            "wrmsr",

            // Enable paging and protected mode in cr0
            "mov rax, cr0",
            "or rax, {}", // PG bit
            "mov cr0, rax",

            in(reg) &gdtr,
            in(reg) cr0_flags,
            options(nostack, preserves_flags)
        );
    }
}*/

pub fn set_segment_selectors() {
    unsafe {
        asm!(
            "mov ax, 0x10",      // data segment selector
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",

            "push 0x08",         // code segment selector (GDT offset for code)
            "lea rax, [2f]",     // load address of label 1
            "push rax",
            "retfq",             // far return to reload CS
            "2:",                // define label 1 here
            options(nostack, preserves_flags),
        );
    }
}

//
// IDT
//

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IdtDescriptor {
    limit: u16,
    base: u64,
}

#[repr(C)]
pub struct InterruptStackFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

#[repr(C, align(16))]
pub struct Idt_Dummy([Entry; 256]);

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Entry {
    offset_low: u16,
    selector: u16,
    options: u16,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl Entry {
    pub fn new(handler: extern "x86-interrupt" fn(InterruptStackFrame)) -> Self {
        let addr = handler as u64;
        Entry {
            offset_low: addr as u16,
            selector: 0x08,
            options: 0x8F00,
            offset_mid: (addr >> 16) as u16,
            offset_high: (addr >> 32) as u32,
            reserved: 0,
        }
    }

    pub fn new_with_error_code(handler: extern "x86-interrupt" fn(InterruptStackFrame, PageFaultErrorCode)) -> Self {
        let handler_addr = handler as u64;
        Entry {
            offset_low: handler_addr as u16,
            selector: 0x08,
            options: 0x8F00,
            offset_mid: (handler_addr >> 16) as u16,
            offset_high: (handler_addr >> 32) as u32,
            reserved: 0,
        }
    }

    pub const fn missing() -> Self {
        Entry {
            offset_low: 0,
            selector: 0,
            options: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }
}

//#[unsafe(naked)]
extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    crate::net::serial::init();

    for &b in b"jezisi kriste" {
        crate::net::serial::write(b);
    }

    loop{}

    /*unsafe {
        core::arch::naked_asm!("iret");
    }*/
}

//#[unsafe(naked)]
extern "x86-interrupt" fn double_fault_handler(_stack_frame: InterruptStackFrame, _error_code: PageFaultErrorCode) {
    crate::net::serial::init();

    for &b in b"jezisi kriste" {
        crate::net::serial::write(b);
    }

    loop{}

    /*unsafe {
        core::arch::naked_asm!("iret");
    }*/
}

//#[unsafe(naked)]
extern "x86-interrupt" fn gpf_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: PageFaultErrorCode,
) {
    crate::net::serial::init();

    for &b in b"jezisi kriste" {
        crate::net::serial::write(b);
    }

    loop{}

    /*unsafe {
        core::arch::naked_asm!("iret");
    }*/
}

extern "x86-interrupt" fn page_fault_handler(_stack_frame: InterruptStackFrame, _error_code: PageFaultErrorCode) {
    crate::net::serial::init();

    for &b in b"jezisi kriste" {
        crate::net::serial::write(b);
    }

    loop{}
}

/*use x86_64::structures::idt::{PageFaultErrorCode, InterruptDescriptorTable};

  static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

  pub fn load_idt() {
  unsafe {
  IDT.breakpoint.set_handler_fn(breakpoint_handler);
  IDT.page_fault.set_handler_fn(page_fault_handler);
  IDT.general_protection_fault.set_handler_fn(gpf_handler);
  IDT.double_fault.set_handler_fn(double_fault_handler);

  IDT.load();
  }
  }*/



// Dummy static IDT
#[unsafe(no_mangle)]
//#[unsafe(link_section = ".rodata")]
pub static mut IDT: Idt_Dummy = Idt_Dummy([Entry::missing(); 256]);

pub fn load_idt() {
    unsafe{
        IDT.0[3] = Entry::new(breakpoint_handler);
        IDT.0[8] = Entry::new_with_error_code(double_fault_handler);
        IDT.0[13] = Entry::new_with_error_code(gpf_handler);
        IDT.0[14] = Entry::new_with_error_code(page_fault_handler);

        let idt_ptr = IdtDescriptor {
            limit: (core::mem::size_of::<Idt_Dummy>() - 1) as u16,
            base: &IDT as *const _ as u64,
        };

        asm!(
            "lidt [{}]",
            in(reg) &idt_ptr,
            options(readonly, nostack, preserves_flags),
        );
    }
}
