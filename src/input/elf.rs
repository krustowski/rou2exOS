use core::{ptr::{copy_nonoverlapping, write_bytes}, u64};

use crate::input::keyboard::keyboard_loop;

#[repr(C)]
#[derive(Debug)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

const PT_LOAD: u32 = 1;

pub unsafe fn load_elf64(elf_addr: usize) -> usize {
    let ehdr = &*(elf_addr as *const Elf64Ehdr);

    rprint!("First 16 bytes (elf_addr + 0x18): ");
    for i in 0..16 {
        rprintn!(*((elf_addr + 0x18)as *const u8).add(i));
        rprint!(" ");
    }
    rprint!("\n");

    // Validate ELF magic
    assert_eq!(&ehdr.e_ident[0..4], b"\x7FELF");
    assert_eq!(ehdr.e_ident[4], 2); // ELF64

    let phdrs = (elf_addr + ehdr.e_phoff as usize) as *const Elf64Phdr;

    for i in 0..ehdr.e_phnum {
        let ph = &*phdrs.add(i as usize);
        if ph.p_type == PT_LOAD {
            let src = (elf_addr + ph.p_offset as usize) as *const u8;
            let dst = ph.p_vaddr as *mut u8;

            // ✅ sane debug info
            rprint!("Loading segment ");
            rprintn!(i);
            rprint!(" to ");
            rprintn!(ph.p_vaddr);
            rprint!(", filesz = ");
            rprintn!(ph.p_filesz);
            rprint!(", memsz = ");
            rprintn!(ph.p_memsz);
            rprint!("\n");

            copy_nonoverlapping(src, dst, ph.p_filesz as usize);
            if ph.p_memsz > ph.p_filesz {
                write_bytes(dst.add(ph.p_filesz as usize), 0, (ph.p_memsz - ph.p_filesz) as usize);
            }
        }
    }

    //rprint!("ELF entry point: ");
    //rprintn!(ehdr.e_entry);
    //rprint!("\n");

    ehdr.e_entry as usize
}

pub type ElfEntry = extern "C" fn() -> u64;

#[no_mangle]
#[link_section = ".data"]
pub static mut saved_kernel_rsp: u64 = 0;

#[no_mangle]
pub unsafe extern "C" fn jump_to_elf(entry: ElfEntry, stack_top: u64, arg: u64) -> () {
    extern "C" {
        fn kernel_return();
    }

    let kernel_rsp: u64;
    core::arch::asm!("mov {}, rsp", out(reg) kernel_rsp);
    saved_kernel_rsp = kernel_rsp;

    // Trampoline
    let user_stack = (stack_top - 8) as *mut u64;
    *user_stack = kernel_return as u64;

    let cs: u16;
    unsafe { core::arch::asm!("mov {0:x}, cs", out(reg) cs) };
    
    println!("Switching to user mode:");
    print!("Current CS: ");
    printn!(cs);
    print!("\n");

    core::arch::asm!(
        "cli",
        "mov ax, 0x23",
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",

        "mov rsp, {0}",
        "mov rdi, {1}",

        //"push 0x08",
        //"push {2}",
        //"pushfq",
        //"push 0x10",
        //"push rsp",
        
        "push 0x23",    
        "push {0}",
        "pushfq",
        "push 0x1B",
        "push {2}",
        "iretq",
        in(reg) user_stack,
        in(reg) arg,
        in(reg) entry,
        options(noreturn)
    );
}

#[no_mangle]
pub unsafe extern "C" fn kernel_return(result: u64) -> ! {
    /*core::arch::asm!(
    // Restore original kernel stack
    "mov rsp, qword ptr [rip + saved_kernel_rsp]",
    "mov rbp, {saved_kernel_rsp}",
    saved_kernel_rsp = in(reg) saved_kernel_rsp,
    options(noreturn),
    );*/

    print!("Program return code: ");
    printn!(result);
    print!("\n");

    // Restore shell
    keyboard_loop();
}

#[no_mangle]
pub unsafe extern "C" fn call_elf(entry: ElfEntry, stack_top: u64, arg: u64) -> u64 {
    let kernel_stack: u64;
    let kernel_stack_pointer: u64;
    let result: u64;

    core::arch::asm!(
        "mov {kernel_stack_pointer}, rbp",
        "mov {kernel_stack}, rsp",
        "mov rsp, {stack}",
        "xor rbp, rbp",
        "call {entry}",
        "mov rbp, {kernel_stack_pointer}",
        "mov rsp, {kernel_stack}",
        kernel_stack = lateout(reg) kernel_stack,
        kernel_stack_pointer = lateout(reg) kernel_stack_pointer,
        stack = in(reg) stack_top,
        entry = in(reg) entry,
        in("rdi") arg,
        lateout("rax") result,
        options(nostack),
    );

    //core::arch::asm!("mov {}, rax", out(reg) result);
    result
}
