use crate::{
    fs::fat12::{block::{Floppy, BlockDevice}, fs::Filesystem, entry::Entry}, 
    input::elf, 
    net::serial,
    //task::process::schedule,
};

const USERLAND_START: u64 = 0x600_000;
const USERLAND_END: u64 = 0x800_000;

enum SyscallReturnCode {
    Okay = 0x00,
    NotImplemented = 0xfb,
    InvalidInput = 0xfc,
    FilesystemError = 0xfd,
    FileNotFound = 0xfe,
    InvalidSyscall = 0xff,
}

/// This function is the syscall ABI dispatching routine. It is called exclusively from the ISR 
/// for interrupt 0x7f. 
#[no_mangle]
pub extern "C" fn syscall_handler() {
    let (syscall_no, arg1, arg2): (u64, u64, u64);
    let mut ret: SyscallReturnCode = SyscallReturnCode::Okay;

    unsafe {
        core::arch::asm!(
            "mov {0}, rax",
            "mov {1}, rdi",
            "mov {2}, rsi",
            out(reg) syscall_no,
            out(reg) arg1,
            out(reg) arg2,
        );
    }

    debug!("syscall_handler: called: ");
    debugn!(syscall_no);
    debug!(", arg1: ");
    debugn!(arg1);
    debug!(", arg2: ");
    debugn!(arg2);
    debug!("\n");

    rprint!("syscall_handler: called: ");
    rprintn!(syscall_no);
    rprint!(", arg1: ");
    rprintn!(arg1);
    rprint!(", arg2: ");
    rprintn!(arg2);
    rprint!("\n");

    match syscall_no {
        /*
         *  Syscall 0x00 --- Program graceful exit
         *
         *  Arg1: program/process/task ID
         *  Arg2: program return code
         */
        0x00 => {
            rprint!("[TASK ");
            rprintn!(arg1);
            rprint!("]: exit\n");

            unsafe {
                core::arch::asm!(
                    "mov rdi, {0}",
                    "mov rsi, {1}",
                    "jmp kernel_return",
                    in(reg) arg1,
                    in(reg) arg2,
                );
            };
        }

        /*
         *  Syscall 0x01 --- Get/Set system info
         *
         *  Arg1: 0x01 or 0x02
         *  Arg2: pointer to system info struct (*mut SysInfo)
         *
         */
        0x01 => {
            if arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let mut sysinfo_ptr = arg2 as *mut SysInfo;

            match arg1 {
                0x01 => {
                    unsafe {
                        let name = b"rou2ex";
                        let user = b"guest";
                        let version = b"v0.9.3";
                        let path = b"/";

                        if let Some(nm) = (*sysinfo_ptr).system_name.get_mut(0..name.len()) {
                            nm.copy_from_slice(name);
                        }

                        if let Some(us) = (*sysinfo_ptr).system_user.get_mut(0..user.len()) {
                            us.copy_from_slice(user);
                        }

                        if let Some(ph) = (*sysinfo_ptr).system_path.get_mut(0..path.len()) {
                            ph.copy_from_slice(path);
                        }

                        if let Some(vn) = (*sysinfo_ptr).system_version.get_mut(0..version.len()) {
                            vn.copy_from_slice(version);
                        }
                    }
                }
                0x02 => {
                    // TODO
                }
                _ => {}
            }

            ret = SyscallReturnCode::Okay;
        }

        /*
         *  Syscall 0x0a --- Allocate memory from heap
         *
         *  Arg1: pointer to type (*mut _)
         *  Arg2: size in bytes to allocate
         */
        0x0a => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            // TODO
            ret = SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x10 --- Print data to standard output
         *
         *  Arg1: pointer to data (&[u8])
         *  Arg2: length in bytes to print
         */
        0x10 => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };

            for &b in slice.iter() {
                if b == b'\0' {
                    break;
                }

                printb!(&[b]);
            }

            ret = SyscallReturnCode::Okay;
        }

        /*
         *  Syscall 0x11 --- Clear the screen (standard output)
         *
         *  Arg1: 0x00
         *  Arg2: 0x00
         */
        0x11 => {
            if arg1 != 0x00 || arg2 != 0x00 {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            clear_screen!();

            ret = SyscallReturnCode::Okay;
        }

        /*
         *  Syscall 0x20 --- Read a file
         *
         *  Arg1: pointer to filename byte slice (&[u8])
         *  Arg2: pointer to buffer (*mut [u8; 512])
         */
        0x20 => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END || arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let name_ptr = arg1 as *const u8;

            let (name, ext) = format_filename(name_ptr);

            let buf_ptr = arg2 as *mut [u8; 512];
            let floppy = Floppy::init();
            let mut file_read: bool = false;

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, | entry | {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 || entry.attr & 0x10 != 0 {
                            return;
                        }

                        unsafe {
                            if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) {
                                return
                            }

                            // Read the file directly into the client's buffer
                            fs.read_file(entry.start_cluster, &mut *buf_ptr);
                            file_read = true;
                        }
                    });

                    if !file_read {
                        ret = SyscallReturnCode::FileNotFound;
                    } else {
                        ret = SyscallReturnCode::Okay;
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }
        }

        /*
         *  Syscall 0x21 --- Write buffer to file
         *
         *  Arg1: pointer to file name, byte slice (*const [u8])
         *  Arg2: pointer to byte buffer (*mut [u8; 512])
         */
        0x21 => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END || arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let name_ptr = arg1 as *const u8;

            let (name, ext) = format_filename(name_ptr);

            let mut filename: [u8; 11] = [b' '; 11];
            filename[0..8].copy_from_slice(&name);
            filename[8..11].copy_from_slice(&ext);

            let buf_ptr = arg2 as *const [u8; 512];
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    unsafe {
                        fs.write_file(0, &filename, &*buf_ptr);
                    }

                    ret = SyscallReturnCode::Okay;
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }

        }

        /*
         *  Syscall 0x22 --- Rename a directory entry
         *
         *  Arg1: pointer to original filename
         *  Arg2: pointer to new filename
         */
        0x22 => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END || arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let (name_old, ext_old) = format_filename(arg1 as *const u8);
            let (name_new, ext_new) = format_filename(arg2 as *const u8);

            let mut filename_old: [u8; 11] = [b' '; 11];
            filename_old[0..8].copy_from_slice(&name_old);
            filename_old[8..11].copy_from_slice(&ext_old);

            let mut filename_new: [u8; 11] = [b' '; 11];
            filename_new[0..8].copy_from_slice(&name_new);
            filename_new[8..11].copy_from_slice(&ext_new);

            let floppy = Floppy::init();
            let mut file_found: bool = false;

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, | entry | {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 {
                            return;
                        }

                        if !entry.name.starts_with(&name_old) || (ext_old.len() > 0 && !entry.ext.starts_with(&ext_old)) {
                            return
                        }

                        // Read the file directly into the client's buffer
                        fs.rename_file(0, &filename_old, &filename_new);
                        file_found = true;
                    });

                    if !file_found {
                        ret = SyscallReturnCode::FileNotFound;
                    } else {
                        ret = SyscallReturnCode::Okay;
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }
        }

        /*
         *  Syscall 0x23 --- Delete a directory entry
         *
         *  Arg1: pointer to original filename
         *  Arg2: 0x00
         */
        0x23 => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END || arg2 != 0 {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let (name, ext) = format_filename(arg1 as *const u8);

            let mut filename: [u8; 11] = [b' '; 11];
            filename[0..8].copy_from_slice(&name);
            filename[8..11].copy_from_slice(&ext);

            let mut file_found: bool = false;
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, | entry | {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 {
                            return;
                        }

                        if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) {
                            return
                        }

                        // Read the file directly into the client's buffer
                        fs.delete_file(0, &filename);
                        file_found = true;
                    });

                    if !file_found {
                        ret = SyscallReturnCode::FileNotFound;
                    } else {
                        ret = SyscallReturnCode::Okay;
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }
        }

        /*
         *  Syscall 0x24 --- Read the FAT table
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to next cluster (*mut u84)
         */
        0x24 => {
            // TODO
            ret = SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x25 --- Write to the FAT table
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to value (*const u84)
         */
        0x25 => {
            // TODO
            ret = SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x26 --- Insert entry into cluster
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to a new directory entry (*const Entry)
         */
        0x26 => {
            // TODO
            ret = SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x27 --- Add new subdirectory
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to a new subdirectory name (*const u8)
         */
        0x27 => {
            if arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let name_ptr = arg2 as *const u8;

            let (name, ext) = format_filename(name_ptr);

            let mut filename: [u8; 11] = [b' '; 11];
            filename[0..8].copy_from_slice(&name);
            filename[8..11].copy_from_slice(&ext);

            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.create_subdirectory(&filename, arg1 as u16);

                    ret = SyscallReturnCode::Okay;
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }
        }

        /*
         *  Syscall 0x28 --- List directory entries
         *  
         *  Arg1: dir cluster No.
         *  Arg2: dir entries pointer (*mut Entry)
         */
        0x28 => {
            let path = arg1 as u16;
            let entries = arg2 as *mut crate::fs::fat12::entry::Entry;

            let mut kentries: [crate::fs::fat12::entry::Entry; 32] = [crate::fs::fat12::entry::Entry::default(); 32];
            let mut offset = 0;

            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(path, |entry| {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.name[0] == 0xFF || entry.attr & 0x08 != 0 {
                            return;
                        }

                        if offset < kentries.len() {
                            kentries[offset] = *entry;
                            offset += 1;
                        }
                    });

                    unsafe {
                        core::ptr::copy_nonoverlapping(kentries.as_ptr(), entries, offset);
                    }

                    ret = SyscallReturnCode::Okay;
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }
        }

        /*
         *  Syscall 0x29 --- Load and run flat binary executable (.BIN)
         *
         *  Arg1: file name
         *  Arg2: pointer to PID (*mut u8)
         */
        0x29 => {
            // TODO
            ret = SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x2A --- Load and run ELF executable (.ELF)
         *
         *  Arg1: file name
         *  Arg2: pointer to PID (*mut u8)
         */
        0x2A => {
            if arg1 < USERLAND_START || arg1 > USERLAND_END || arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let (name, ext) = format_filename(arg1 as *const u8);
            let mut pid = arg2 as *mut u8;

            let mut file_found: bool = false;
            let mut file_size = 0;
            let mut cluster = 0;

            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, | entry | {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 || entry.attr & 0x10 != 0 {
                            return;
                        }

                        if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) || &ext != b"ELF" {
                            return
                        }

                        file_found = true;
                        file_size = entry.file_size;
                        cluster = entry.start_cluster;

                        //
                        //  Load the whole ELF to a temp location
                        //

                        let load_addr: u64 = 0x650_000;
                        let stack_top = 0x670_000;

                        let mut offset = 0;

                        while file_size - offset > 0 {
                            let lba = fs.cluster_to_lba(cluster);
                            let mut sector = [0u8; 512];

                            fs.device.read_sector(lba, &mut sector);

                            let dst = load_addr as *mut u8;

                            rprint!("Loading ELF image to memory segment\n");

                            unsafe {
                                for i in 0..512 {
                                    if let Some(byte) = sector.get(i) {
                                        *dst.add(i + offset as usize) = *byte;
                                    }
                                }
                            }

                            cluster = fs.read_fat12_entry(cluster);

                            if cluster >= 0xFF8 || cluster == 0 {
                                break;
                            }

                            offset += 512;
                        }

                        let arg: u64 = 555;
                        let entry_ptr = (load_addr + 0x18) as *const u8;

                        unsafe {
                            // Assume `elf_image` is a pointer to the loaded ELF file in memory
                            let entry_addr = elf::load_elf64(load_addr as usize);

                            // Cast and jump
                            let entry_fn: extern "C" fn() -> u64 = core::mem::transmute(entry_addr as *const ());

                            rprint!("Jumping to the program entry point...\n");
                            elf::jump_to_elf(entry_fn, stack_top, arg);
                        }

                        ret = SyscallReturnCode::Okay;

                    });
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = SyscallReturnCode::FilesystemError;
                }
            }

            if !file_found {
                ret = SyscallReturnCode::FileNotFound;
                return;
            } 

        }

        /*
         *  Syscall 0x30 --- Send value to port
         *
         *  Arg1: port ID
         *  Arg2: pointer to value (u64)
         */
        0x30 => {
            let port = arg1 as *const u16;
            let value = arg2 as *const u32;

            unsafe {
                crate::input::port::write_u32(*port, *value);
            }

            ret = SyscallReturnCode::Okay;
        }

        /*
         *  Syscall 0x31 --- Read value from port
         *
         *  Arg1: port ID
         *  Arg2: pointer to value (u64)
         */
        0x31 => {
            if arg2 < USERLAND_START || arg2 > USERLAND_END {
                ret = SyscallReturnCode::InvalidInput;
                return;
            }

            let port = arg1 as *const u16;
            let value = arg2 as *mut u32;

            unsafe {
                *value = crate::input::port::read_u32(*port);
            }

            ret = SyscallReturnCode::Okay;
        }

        /*
         *  Syscall 0x32 --- Serial port (UART) handling
         *
         *  Arg1: op code
         *  Arg2: pointer to value (*mut u32)
         */
        0x32 => {
            match arg1 {
                0x01 => {
                    // Serial init
                    if arg2 != 0x00 {
                        ret = SyscallReturnCode::InvalidInput;
                        return;
                    }

                    serial::init();

                    ret = SyscallReturnCode::Okay;
                }
                
                // Read from UART
                0x02 => {
                    if arg2 < USERLAND_START || arg2 > USERLAND_END {
                        ret = SyscallReturnCode::InvalidInput;
                        return;
                    }

                    let mut value = arg2 as *mut u32;

                    unsafe {
                        *value = serial::read() as u32;
                    }

                    ret = SyscallReturnCode::Okay;
                }

                // Write to UART
                0x03 => {
                    if arg2 < USERLAND_START || arg2 > USERLAND_END {
                        ret = SyscallReturnCode::InvalidInput;
                        return;
                    }

                    let mut value = arg2 as *const u32;

                    unsafe {
                        serial::write(*value as u8);
                    }

                    ret = SyscallReturnCode::Okay;
                }

                _ => {
                    ret = SyscallReturnCode::InvalidInput;
                }
            }
        }
        /*
         *  Syscall 0x33 --- Create a packet
         *
         *  Arg1: packet type
         *  Arg2: pointer to buffer (*mut u8)
         */
        0x33 => {}

        /*
         *  Syscall 0x34 --- Send a packet
         *
         *  Arg1: packet type
         *  Arg2: pointer to buffer (*mut u8)
         */
        0x34 => {}

        /*
         *  Unknown syscall
         */
        _ => {
            rprint!("Unknown syscall: ");
            rprintn!(syscall_no);
            rprint!("\n");

            ret = SyscallReturnCode::InvalidSyscall;
        }
    }

    // Write the response code
    unsafe {
        core::arch::asm!(
            "mov rax, {0:r}",
            in(reg) (ret as u64),
        );
    }
}

fn format_filename(name_ptr: *const u8) -> ([u8; 8], [u8; 3]) {
    let mut name = [b' '; 8];
    let mut ext = [b' '; 3];

    unsafe {
        let mut i = 0;
        let mut saw_dot = false;
        let mut ext_i = 0;

        while *name_ptr.add(i) != 0 {
            let c = *name_ptr.add(i);
            if c == b'.' {
                saw_dot = true;
                i += 1;
                continue;
            }

            if !saw_dot {
                if i < 8 {
                    name[i] = c.to_ascii_uppercase();
                }
            } else {
                if ext_i < 3 {
                    ext[ext_i] = c.to_ascii_uppercase();
                    ext_i += 1;
                }
            }

            i += 1;
        }

        (name, ext)
    }
}

//
//
//

#[no_mangle]
pub extern "C" fn syscall_80h() {
    let code: u64;

    //schedule();
    return;

    unsafe {
        core::arch::asm!(
            "mov {0}, rax",
            //"mov {1}, rdi",
            //"mov {2}, rsi",
            out(reg) code,
            //out(reg) arg1,
            //out(reg) arg2,
        );
    }

    match code {
        0x01 => {
            // EXIT USER MODE
            unsafe {
                core::arch::asm!("iretq");
            }
        }
        _ => {
            unsafe {
                core::arch::asm!("mov rax, 0xff");
            }
        }
    }
}

#[repr(C, packed)]
#[derive(Default)]
pub struct SysInfo {
    pub system_name: [u8; 32],
    pub system_user: [u8; 32],
    pub system_path: [u8; 32],
    pub system_version: [u8; 8],
    pub system_uptime: u32,
}

