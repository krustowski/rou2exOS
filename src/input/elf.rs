use core::ptr::{copy_nonoverlapping, write_bytes};

use crate::fs::block::BlockDevice;
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

/// Lowest virtual address a userland ELF segment may occupy.
const USERLAND_START: u64 = 0x600_000;
/// Highest virtual address (exclusive) a userland ELF segment may occupy.
const USERLAND_END: u64 = 0xA00_000;

pub unsafe fn load_elf64(elf_addr: usize) -> usize {
    let ehdr = &*(elf_addr as *const Elf64Ehdr);

    rprint!("First 16 bytes (elf_addr + 0x18): ");
    for i in 0..16 {
        rprintn!(*((elf_addr + 0x18) as *const u8).add(i));
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
            // Reject any segment that would land outside the user-accessible
            // region.  This prevents a malformed ELF from scribbling over
            // kernel memory.
            let seg_end = ph.p_vaddr.saturating_add(ph.p_memsz);
            if ph.p_vaddr < USERLAND_START || seg_end > USERLAND_END {
                rprint!("ELF segment ");
                rprintn!(i);
                rprint!(" vaddr=");
                rprintn!(ph.p_vaddr);
                rprint!(" is outside userland — skipping\n");
                continue;
            }

            let src = (elf_addr + ph.p_offset as usize) as *const u8;
            let dst = ph.p_vaddr as *mut u8;

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
                write_bytes(
                    dst.add(ph.p_filesz as usize),
                    0,
                    (ph.p_memsz - ph.p_filesz) as usize,
                );
            }
        }
    }

    ehdr.e_entry as usize
}

#[derive(PartialEq, Clone, Copy)]
pub enum RunMode {
    Foreground,
    Background,
}

static mut STACK_NO: usize = 0;

use crate::fs::fat12::{block::Floppy, fs::Filesystem};

pub fn run_elf(filename_input: &[u8], _args: &[u8], mode: RunMode) -> bool {
    if filename_input.is_empty() || filename_input.len() > 8 {
        return false;
    }

    // 12 = filename + ext + dot
    let mut filename = [b' '; 12];

    /*if let Some(slice) = filename.get_mut(..filename_input.len()) {
        slice.copy_from_slice(filename_input);
    }*/
    if let Some(slice) = filename.get_mut(9..12) {
        slice.copy_from_slice(b"ELF");
    }

    let floppy = Floppy::init();

    // Init the filesystem to look for a match
    match Filesystem::new(&floppy) {
        Ok(fs) => {
            unsafe {
                let mut cluster: u16 = 0;
                let mut offset = 0;
                let mut size = 0;

                fs.for_each_entry(crate::init::config::PATH_CLUSTER, |entry| {
                    if entry.name.starts_with(filename_input) && entry.ext.starts_with(b"ELF") {
                        cluster = entry.start_cluster;
                        size = entry.file_size;

                        if let Some(slice) = filename.get_mut(0..8) {
                            slice.copy_from_slice(&entry.name);
                        }
                    }
                });

                if cluster == 0 {
                    error!("no such file found");
                    error!();
                    return false;
                }

                rprint!("Size: ");
                rprintn!(size);
                rprint!("\n");

                let addrs = [0x640_000, 0x680_000, 0x6c0_000, 0x700_000, 0x720_000];
                let load_addr: u64 = addrs[STACK_NO % 5];

                while size - offset > 0 {
                    let lba = fs.cluster_to_lba(cluster);
                    let mut sector = [0u8; 512];

                    fs.device.read_sector(lba, &mut sector);

                    let dst = load_addr as *mut u8;

                    rprint!("Loading ELF image to memory segment\n");
                    for i in 0..512 {
                        if let Some(byte) = sector.get(i) {
                            *dst.add(i + offset as usize) = *byte;
                        }
                    }

                    cluster = fs.read_fat12_entry(cluster);

                    if cluster >= 0xFF8 || cluster == 0 {
                        break;
                    }

                    offset += 512;
                }

                let entry_ptr = (load_addr + 0x18) as *const u8;

                rprint!("First 16 bytes (load_addr + 0x18): ");
                for i in 0..16 {
                    rprintn!(*entry_ptr.add(i));
                    rprint!(" ");
                }
                rprint!("\n");

                // Parse and copy ELF segments to their p_vaddr destinations.
                let entry_addr = load_elf64(load_addr as usize);

                rprint!("ELF entry point: ");
                rprintn!(entry_addr);
                rprint!("\n");

                // Ensure the User bit is set in the active page table for the
                // entire userland region so CPL=3 code can access its own
                // pages.  This is belt-and-suspenders on top of boot.asm but
                // makes the mapping explicit and survives any future boot.asm
                // changes.
                crate::mem::pages::ensure_user_pages(USERLAND_START, USERLAND_END);

                // Stacks live at the top of the user-accessible region
                // (0x400000–0x9FFFFF, user-flag set in boot.asm).  Each slot
                // gets 256 KB of stack space, starting well above where code
                // typically loads (0x600000+), so normal stack growth never
                // collides with ELF segments.
                let stacks = [0x8C0_000u64, 0x8A0_000, 0x880_000, 0x860_000, 0x840_000];
                let stack_top = stacks[STACK_NO % 5];
                STACK_NO += 1;

                // cast and jump
                //let entry_fn: extern "C" fn() -> ! = core::mem::transmute(entry_addr as *const ());

                let mut name: [u8; 16] = [b' '; 16];

                if let Some(slice) = name.get_mut(0..12) {
                    slice.copy_from_slice(&filename[0..12]);
                }

                // Create a new process to be run
                let pid = crate::task::scheduler::new_process(
                    name,
                    crate::task::process::Mode::User,
                    entry_addr as u64,
                    stack_top,
                );

                if pid == 0xff || pid == 0x00 {
                    rprint!("Error starting new process...\n");
                    error!("Error starting new process...\n\n");
                    return false;
                }

                match mode {
                    RunMode::Background => {}
                    RunMode::Foreground => {
                        // Make the kernel shell idle
                        crate::task::scheduler::idle(0xff);
                    }
                }
            }
        }
        Err(e) => {
            error!(e);
            error!();
            return false;
        }
    }
    true
}

pub type ElfEntry = extern "C" fn() -> u64;

#[no_mangle]
#[link_section = ".data"]
pub static mut SAVED_KERNEL_RSP: u64 = 0;

#[expect(clippy::fn_to_numeric_cast)]
#[no_mangle]
pub unsafe extern "C" fn jump_to_elf(entry: ElfEntry, stack_top: u64, arg: u64) {
    extern "C" {
        fn kernel_return();
    }

    let kernel_rsp: u64;
    core::arch::asm!("mov {}, rsp", out(reg) kernel_rsp);
    SAVED_KERNEL_RSP = kernel_rsp;

    // Trampoline
    let user_stack = (stack_top - 8) as *mut u64;
    *user_stack = kernel_return as u64;

    println!("Switching to user mode:");

    core::arch::asm!(
        //"cli",
        "mov rsp, {0}",
        "mov rdi, {1}",

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
    let _kernel_stack: u64;
    let _kernel_stack_pointer: u64;
    let result: u64;

    core::arch::asm!(
        "mov {kernel_stack_pointer}, rbp",
        "mov {kernel_stack}, rsp",
        "mov rsp, {stack}",
        "xor rbp, rbp",
        "call {entry}",
        "mov rbp, {kernel_stack_pointer}",
        "mov rsp, {kernel_stack}",
        kernel_stack = lateout(reg) _kernel_stack,
        kernel_stack_pointer = lateout(reg) _kernel_stack_pointer,
        stack = in(reg) stack_top,
        entry = in(reg) entry,
        in("rdi") arg,
        lateout("rax") result,
        options(nostack),
    );

    //core::arch::asm!("mov {}, rax", out(reg) result);
    result
}
