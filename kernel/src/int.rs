use core::arch::asm;
use x86_64::structures::idt::InterruptStackFrame;

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

static GDT: Gdt = Gdt([
    0,
    0x00cf9a000000ffff, // 64-bit code: base=0, limit=0xFFFFF, flags
    0x00cf92000000ffff, // 64-bit data
]);

fn get_gdtr() -> GdtDescriptor {
    GdtDescriptor {
        limit: (core::mem::size_of::<Gdt>() - 1) as u16,
        base: unsafe { &GDT as *const _ as u64 },
    }
}

pub fn load_gdt() {
    let gdtr = get_gdtr();
    unsafe {
        asm!(
            "lgdt [{}]",
            in(reg) &gdtr,
            options(nostack, preserves_flags)
        );
    }
}

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
//
//

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IdtDescriptor {
    limit: u16,
    base: u64,
}

#[repr(C, align(16))]
pub struct Idt([Entry; 256]);

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
    pub fn new(handler: extern "C" fn(&mut InterruptStackFrame)) -> Self {
        let addr = handler as u64;
        Entry {
            offset_low: addr as u16,
            selector: 0x08,
            options: 0x8E00,
            offset_mid: (addr >> 16) as u16,
            offset_high: (addr >> 32) as u32,
            reserved: 0,
        }
    }

    pub fn new_with_error_code(handler: extern "C" fn(&mut InterruptStackFrame, u64)) -> Self {
        let handler_addr = handler as u64;
        Entry {
            offset_low: handler_addr as u16,
            selector: 0x08,
            options: 0x8E00,
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

extern "C" fn breakpoint_handler(_stack_frame: &mut InterruptStackFrame) {
    unsafe {
        asm!("iret");
    }
}

extern "C" fn double_fault_handler(_stack_frame: &mut InterruptStackFrame, _error_code: u64) {
    unsafe {
        asm!("iret");
    }
}

extern "C" fn gpf_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    unsafe {
        asm!("iret");
    }
}

extern "C" fn page_fault_handler(
    _stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) {
    unsafe {
        asm!("iret");
    }
}

// Dummy static IDT
#[unsafe(no_mangle)]
#[unsafe(link_section = ".rodata")]
pub static mut IDT: Idt = Idt([Entry::missing(); 256]);

pub fn load_idt() {
    unsafe{
        IDT.0[3] = Entry::new(breakpoint_handler);
        IDT.0[8] = Entry::new_with_error_code(double_fault_handler);
        IDT.0[13] = Entry::new_with_error_code(gpf_handler);
        IDT.0[14] = Entry::new_with_error_code(page_fault_handler);

        let idt_ptr = IdtDescriptor {
            limit: (core::mem::size_of::<Idt>() - 1) as u16,
            base: &IDT as *const _ as u64,
        };

        asm!(
            "lidt [{}]",
            in(reg) &idt_ptr,
            options(readonly, nostack, preserves_flags),
        );
    }
}

