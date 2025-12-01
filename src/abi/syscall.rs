use core::arch::naked_asm;

use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    fs::fat12::{
        block::{BlockDevice, Floppy},
        fs::Filesystem,
    },
    input::{elf, irq},
    net::{icmp, ipv4, serial, tcp},
    //task::process::schedule,
    time::rtc,
};

const USERLAND_START: u64 = 0x600_000;
const USERLAND_END: u64 = 0x800_000;

#[repr(u64)]
enum SyscallReturnCode {
    Ok = 0x00,
    NotImplemented = 0xfb,
    InvalidInput = 0xfc,
    FilesystemError = 0xfd,
    FileNotFound = 0xfe,
    InvalidSyscall = 0xff,
}

/// This function is the syscall ABI dispatching routine. It is called exclusively from the ISR
/// for interrupt 0x7f.
#[unsafe(naked)]
pub extern "x86-interrupt" fn syscall_handler(_: InterruptStackFrame) {
    naked_asm!(
        "mov rcx, rdx",
        "cld",
        "call {syscall}",
        "iretq",
        syscall = sym syscall_inner,
    )
}

extern "C" fn syscall_inner(arg1: u64, arg2: u64, syscall_no: u64) -> SyscallReturnCode {
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
                    in(reg) arg2,
                    in(reg) arg1,
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
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            let sysinfo_ptr = arg2 as *mut SysInfo;

            match arg1 {
                0x01 => unsafe {
                    let name = b"rou2ex";
                    let user = b"guest";
                    let version = b"v0.10.1";
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
                },
                0x02 => {
                    // TODO
                }
                _ => {}
            }
        }

        /*
         *  Syscall 0x02 --- Get the RTC time
         *
         *  Arg1: 0x01
         *  Arg2: pointer to RTC structu (*mut RTC)
         */
        0x02 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            if arg1 == 0x01 {
                let rtc_data = arg2 as *mut RTC;

                unsafe {
                    (
                        (*rtc_data).year,
                        (*rtc_data).month,
                        (*rtc_data).day,
                        (*rtc_data).hours,
                        (*rtc_data).minutes,
                        (*rtc_data).seconds,
                    ) = rtc::read_rtc_full();
                }
            }
        }

        /*
         *  Syscall 0x03 --- Pipe subscription handling
         *
         *  Arg1: op type
         *  Arg2: pointer to circular buffer (*const u8)
         */
        0x03 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            match arg1 {
                0x01 => {
                    irq::pipe_subscribe(arg2);
                }

                0x02 => {
                    irq::pipe_unsubscribe(arg2);
                }

                0x03 => {
                    unsafe {
                        #[expect(static_mut_refs)] // this is bad but i cant figure out how to fix
                        for s in irq::RECEPTORS.iter() {
                            if s.pid == 123 {
                                // Try copy immediately
                                let copied = s.copy_to_user(arg2 as *mut u8, 16);
                                if copied > 0 {
                                    break;
                                }

                                // No data: block current process until dispatcher wakes it
                                //block_current_process_on_keyboard();
                                // After wake, try copy again

                                s.copy_to_user(arg2 as *mut u8, 16);
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        /*
         *  Syscall 0x0a --- Allocate memory from heap
         *
         *  Arg1: pointer to type (*mut _)
         *  Arg2: size in bytes to allocate
         */
        0x0a => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput;
            }

            // TODO
            return SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x10 --- Print data to standard output
         *
         *  Arg1: pointer to data (&[u8])
         *  Arg2: length in bytes to print
         */
        0x10 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput;
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
        }

        /*
         *  Syscall 0x11 --- Clear the screen (standard output)
         *
         *  Arg1: 0x00
         *  Arg2: 0x00
         */
        0x11 => {
            if arg1 != 0x00 || arg2 != 0x00 {
                return SyscallReturnCode::InvalidInput;
            }

            clear_screen!();
        }

        /*
         *  Syscall 0x12 --- Write graphical pixel
         *
         *  Arg1: encoded position
         *  Arg2: encoded color
         */
        0x12 => unsafe {
            let x = arg1 as u32 >> 16;
            let y = (arg1 & 0xffff) as u32;

            let bg = arg2 as u16 >> 8;
            let fg = arg2 as u16 & 0xff;

            let color = 0x00ffff00;

            let fb = crate::init::check::FRAMEBUFFER_PTR;
            let ptr = fb.addr as *mut u64;

            rprint!("X: ");
            rprintn!(x);
            rprint!(", Y: ");
            rprintn!(y);
            rprint!(", FB: ");
            rprintn!(fb.addr);
            rprint!("\n");

            for x0 in 0..100 {
                let offset = y * (fb.pitch / 4) + x + x0;

                ptr.add(offset as usize).write_volatile(color);
            }
        },

        /*
         *  Syscall 0x1a --- Play a frequency
         *
         *  Arg2: frequency in Hz
         *  Arg2: duration in ms
         */
        0x1a => {
            if !(20..=20_000).contains(&arg1) {
                return SyscallReturnCode::InvalidInput;
            }

            crate::audio::beep::beep(arg1 as u32);
            crate::audio::midi::wait_millis(arg2 as u16);
            crate::audio::beep::stop_beep();
        }

        /*
         *  Syscall 0x1b --- Play an audio file
         *
         *  Arg2: audio file type
         *  Arg2: pointer to file name (*const u8)
         */
        0x1b => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            let name_ptr = arg1 as *const u8;
            let (name, ext) = format_filename(name_ptr);

            let mut buf: [u8; 4096] = [0u8; 4096];
            let mut file_found = false;

            match arg1 {
                // MIDI file format 0
                0x01 => {
                    let floppy = Floppy::init();

                    match Filesystem::new(&floppy) {
                        Ok(fs) => {
                            fs.for_each_entry(0, |entry| {
                                if entry.name[0] == 0x00
                                    || entry.name[0] == 0xE5
                                    || entry.attr & 0x08 != 0
                                    || entry.attr & 0x10 != 0
                                {
                                    return;
                                }

                                if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) {
                                    return;
                                }

                                // Read the file directly into the client's buffer
                                fs.read_file(entry.start_cluster, &mut buf);
                                file_found = true;
                            });
                        }
                        Err(e) => {
                            rprint!(e);
                            rprint!("\n");
                        }
                    }

                    if !file_found {
                        return SyscallReturnCode::FileNotFound;
                    }

                    if let Some(midi) = crate::audio::midi::parse_midi_format0(&buf) {
                        crate::audio::midi::play_midi(&midi);
                        crate::audio::beep::stop_beep();
                    } else {
                        return SyscallReturnCode::FilesystemError;
                    }
                }

                _ => {
                    return SyscallReturnCode::InvalidInput;
                }
            }
        }

        /*
         *  Syscall 0x1f --- Stop the speaker
         *
         *  Arg2: 0x00
         *  Arg2: 0x00
         */
        0x1f => {
            if arg1 != 0x00 || arg2 != 0x00 {
                return SyscallReturnCode::InvalidInput;
            }

            crate::audio::beep::stop_beep();
        }

        /*
         *  Syscall 0x20 --- Read a file
         *
         *  Arg1: pointer to filename byte slice (&[u8])
         *  Arg2: pointer to buffer (*mut [u8; 512])
         */
        0x20 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1)
                || !(USERLAND_START..=USERLAND_END).contains(&arg2)
            {
                return SyscallReturnCode::InvalidInput;
            }

            let name_ptr = arg1 as *const u8;

            let (name, ext) = format_filename(name_ptr);

            let buf_ptr = arg2 as *mut u8;
            let floppy = Floppy::init();
            let mut file_read: bool = false;

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, |entry| {
                        if entry.name[0] == 0x00
                            || entry.name[0] == 0xE5
                            || entry.attr & 0x08 != 0
                            || entry.attr & 0x10 != 0
                        {
                            return;
                        }

                        if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) {
                            return;
                        }

                        let mut cluster = entry.start_cluster;
                        let mut offset = 0;

                        while entry.file_size - offset > 0 {
                            let lba = fs.cluster_to_lba(cluster);
                            let mut sector = [0u8; 512];

                            fs.device.read_sector(lba, &mut sector);

                            let dst = buf_ptr;

                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    sector.as_ptr(),
                                    dst.add(offset as usize),
                                    512,
                                );
                            }

                            cluster = fs.read_fat12_entry(cluster);

                            if cluster >= 0xFF8 || cluster == 0 {
                                break;
                            }

                            offset += 512;
                        }
                        file_read = true;
                    });

                    if !file_read {
                        return SyscallReturnCode::FileNotFound;
                    } else {
                        return SyscallReturnCode::Ok;
                    }
                }

                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
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
            if !(USERLAND_START..=USERLAND_END).contains(&arg1)
                || !(USERLAND_START..=USERLAND_END).contains(&arg2)
            {
                return SyscallReturnCode::InvalidInput;
            }

            let name_ptr = arg1 as *const u8;

            let (name, ext) = format_filename(name_ptr);

            let mut filename: [u8; 11] = [b' '; 11];
            filename[0..8].copy_from_slice(&name);
            filename[8..11].copy_from_slice(&ext);

            let buf_ptr = arg2 as *const [u8; 512];
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => unsafe {
                    fs.write_file(0, &filename, &*buf_ptr);
                },
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
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
            if !(USERLAND_START..=USERLAND_END).contains(&arg1)
                || !(USERLAND_START..=USERLAND_END).contains(&arg2)
            {
                return SyscallReturnCode::InvalidInput;
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
                    fs.for_each_entry(0, |entry| {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0
                        {
                            return;
                        }

                        if !entry.name.starts_with(&name_old)
                            || (!ext_old.is_empty() && !entry.ext.starts_with(&ext_old))
                        {
                            return;
                        }

                        // Read the file directly into the client's buffer
                        fs.rename_file(0, &filename_old, &filename_new);
                        file_found = true;
                    });

                    if !file_found {
                        return SyscallReturnCode::FileNotFound;
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
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
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) || arg2 != 0 {
                return SyscallReturnCode::InvalidInput;
            }

            let (name, ext) = format_filename(arg1 as *const u8);

            let mut filename: [u8; 11] = [b' '; 11];
            filename[0..8].copy_from_slice(&name);
            filename[8..11].copy_from_slice(&ext);

            let mut file_found: bool = false;
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, |entry| {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0
                        {
                            return;
                        }

                        if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) {
                            return;
                        }

                        // Read the file directly into the client's buffer
                        fs.delete_file(0, &filename);
                        file_found = true;
                    });

                    if !file_found {
                        return SyscallReturnCode::FileNotFound;
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
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
            return SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x25 --- Write to the FAT table
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to value (*const u84)
         */
        0x25 => {
            // TODO
            return SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x26 --- Insert entry into cluster
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to a new directory entry (*const Entry)
         */
        0x26 => {
            // TODO
            return SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x27 --- Add new subdirectory
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to a new subdirectory name (*const u8)
         */
        0x27 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
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
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
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

            let mut kentries: [crate::fs::fat12::entry::Entry; 32] =
                [crate::fs::fat12::entry::Entry::default(); 32];
            let mut offset = 0;

            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(path, |entry| {
                        if entry.name[0] == 0x00
                            || entry.name[0] == 0xE5
                            || entry.name[0] == 0xFF
                            || entry.attr & 0x08 != 0
                        {
                            return;
                        }

                        if let Some(entry_mut) = kentries.get_mut(offset) {
                            *entry_mut = *entry;
                            offset += 1;
                        }
                    });

                    unsafe {
                        core::ptr::copy_nonoverlapping(kentries.as_ptr(), entries, offset);
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
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
            return SyscallReturnCode::NotImplemented;
        }

        /*
         *  Syscall 0x2A --- Load and run ELF executable (.ELF)
         *
         *  Arg1: file name
         *  Arg2: pointer to PID (*mut u8)
         */
        0x2A => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1)
                || !(USERLAND_START..=USERLAND_END).contains(&arg2)
            {
                return SyscallReturnCode::InvalidInput;
            }

            let (name, ext) = format_filename(arg1 as *const u8);

            let mut file_found: bool = false;
            let mut file_size = 0;
            let mut cluster = 0;

            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, |entry| {
                        if entry.name[0] == 0x00
                            || entry.name[0] == 0xE5
                            || entry.attr & 0x08 != 0
                            || entry.attr & 0x10 != 0
                        {
                            return;
                        }

                        if !entry.name.starts_with(&name)
                            || !entry.ext.starts_with(&ext)
                            || &ext != b"ELF"
                        {
                            return;
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
                        // let entry_ptr = (load_addr + 0x18) as *const u8;

                        unsafe {
                            // Assume `elf_image` is a pointer to the loaded ELF file in memory
                            let entry_addr = elf::load_elf64(load_addr as usize);

                            // Cast and jump
                            let entry_fn: extern "C" fn() -> u64 =
                                core::mem::transmute(entry_addr as *const ());

                            rprint!("Jumping to the program entry point...\n");
                            elf::jump_to_elf(entry_fn, stack_top, arg);
                        }
                    });
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError;
                }
            }

            if !file_found {
                return SyscallReturnCode::FileNotFound;
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
        }

        /*
         *  Syscall 0x31 --- Read value from port
         *
         *  Arg1: port ID
         *  Arg2: pointer to value (u64)
         */
        0x31 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            let port = arg1 as *const u16;
            let value = arg2 as *mut u32;

            unsafe {
                *value = crate::input::port::read_u32(*port);
            }
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
                        return SyscallReturnCode::InvalidInput;
                    }

                    serial::init();
                }

                // Read from UART
                0x02 => {
                    if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                        return SyscallReturnCode::InvalidInput;
                    }

                    let value = arg2 as *mut u32;

                    unsafe {
                        *value = serial::read() as u32;
                    }
                }

                // Write to UART
                0x03 => {
                    if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                        return SyscallReturnCode::InvalidInput;
                    }

                    let value = arg2 as *const u32;

                    unsafe {
                        serial::write(*value as u8);
                    }
                }

                _ => {
                    return SyscallReturnCode::InvalidInput;
                }
            }
        }

        /*
         *  Syscall 0x33 --- Create a packet
         *
         *  Arg1: packet type
         *  Arg2: pointer to buffer (*mut u8)
         */
        0x33 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            match arg1 {
                // IPv4 packet
                0x01 => {
                    let packet = arg2 as *mut u8;
                    let header = packet as *mut ipv4::Ipv4Header;
                    let mut ipv4_buffer = [0u8; 1500];
                    let mut ipv4_buffer_aux = [0u8; 1500];

                    unsafe {
                        let header_len = ((*header).version_ihl & 0x0F) * 4;
                        let total_len = u16::from_be((*header).total_length);

                        if total_len >= 1500 {
                            return SyscallReturnCode::InvalidInput;
                        }

                        core::ptr::copy_nonoverlapping(
                            packet,
                            ipv4_buffer_aux.as_mut_ptr(),
                            (header_len as u16 + total_len) as usize,
                        );

                        let payload = ipv4_buffer_aux
                            .get(header_len as usize..total_len as usize)
                            .unwrap_or(&[]);

                        let ipv4_len = ipv4::create_packet(
                            (*header).dest_ip,
                            (*header).source_ip,
                            (*header).protocol,
                            payload,
                            &mut ipv4_buffer,
                        );

                        if ipv4_len == 0 {
                            return SyscallReturnCode::InvalidInput;
                        }

                        let ipv4_slice = ipv4_buffer.get(..ipv4_len).unwrap_or(&[]);

                        let zeros = [0u8; 512];

                        core::ptr::copy(zeros.as_ptr(), packet, zeros.len());
                        core::ptr::copy(ipv4_slice.as_ptr(), packet, ipv4_len);
                    }
                }

                // ICMP packet
                0x02 => {
                    let packet = arg2 as *mut u8;
                    let header = packet as *mut icmp::IcmpHeader;
                    let mut icmp_buffer = [0u8; 64];
                    let mut icmp_buffer_aux = [0u8; 64];

                    unsafe {
                        core::ptr::copy_nonoverlapping(packet, icmp_buffer_aux.as_mut_ptr(), 1500);

                        let payload = icmp_buffer_aux.get(8..).unwrap_or(&[]);

                        let icmp_len = icmp::create_packet(
                            0,
                            (*header).identifier,
                            (*header).sequence_number,
                            payload,
                            &mut icmp_buffer,
                        );
                        let icmp_slice = icmp_buffer.get(..icmp_len).unwrap_or(&[]);

                        core::ptr::copy_nonoverlapping(icmp_slice.as_ptr(), packet, icmp_len);
                    }
                }

                // TCP packet
                0x03 => {
                    let packet = arg2 as *mut u8;
                    let request = packet as *mut TcpPacketRequest;
                    let mut tcp_buffer = [0u8; 1400];
                    let mut tcp_buffer_aux = [0u8; 1400];

                    //let tcp_header_len = core::mem::size_of::<tcp::TcpHeader>() as usize;
                    let tcp_req_len = core::mem::size_of::<TcpPacketRequest>();

                    unsafe {
                        //core::ptr::copy_nonoverlapping(packet, tcp_buffer.as_mut_ptr(), tcp_req_len + (*request).length as usize);
                        core::ptr::copy_nonoverlapping(
                            packet,
                            tcp_buffer_aux.as_mut_ptr(),
                            tcp_req_len + (*request).length as usize,
                        );

                        let payload = tcp_buffer_aux
                            .get(tcp_req_len..tcp_req_len + (*request).length as usize)
                            .unwrap_or(&[]);

                        let tcp_len = tcp::create_packet(
                            (*request).header.source_port,
                            (*request).header.dest_port,
                            (*request).header.seq_num,
                            (*request).header.ack_num,
                            (*request).header.data_offset_reserved_flags & 0xFF,
                            1024,
                            payload,
                            (*request).src_ip,
                            (*request).dst_ip,
                            &mut tcp_buffer,
                        );
                        let tcp_slice = tcp_buffer.get(0..tcp_len).unwrap_or(&[]);

                        let zeros = [0u8; 512];

                        core::ptr::copy(zeros.as_ptr(), packet, zeros.len());
                        core::ptr::copy(tcp_slice.as_ptr(), packet, tcp_len);
                    }
                }

                _ => {}
            }
        }

        /*
         *  Syscall 0x34 --- Send a packet
         *
         *  Arg1: packet type
         *  Arg2: pointer to buffer (*const u8)
         */
        0x34 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput;
            }

            if arg1 == 0x01 {
                let packet = arg2 as *const u8;

                let header = packet as *const ipv4::Ipv4Header;

                unsafe {
                    let total_len = u16::from_be((*header).total_length);
                    let slice = core::slice::from_raw_parts(packet, total_len as usize);

                    ipv4::send_packet(slice);
                }
            }
        }

        /*
         *  Unknown syscall
         */
        _ => {
            rprint!("Unknown syscall: ");
            rprintn!(syscall_no);
            rprint!("\n");

            return SyscallReturnCode::InvalidSyscall;
        }
    }

    SyscallReturnCode::Ok
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
                if let Some(ch) = name.get_mut(i) {
                    *ch = c.to_ascii_uppercase();
                }
            } else if let Some(ch) = ext.get_mut(ext_i) {
                *ch = c.to_ascii_uppercase();
                ext_i += 1;
            }

            i += 1;
        }

        (name, ext)
    }
}

pub extern "x86-interrupt" fn syscall_80h(_stack: InterruptStackFrame) {
    //schedule();

    // unsafe {
    //     core::arch::asm!(
    //         "mov {0}, rax",
    //         //"mov {1}, rdi",
    //         //"mov {2}, rsi",
    //         out(reg) code,
    //         //out(reg) arg1,
    //         //out(reg) arg2,
    //     );
    // }

    // match code {
    //     0x01 => {
    //         // EXIT USER MODE
    //         unsafe {
    //             core::arch::asm!("iretq");
    //         }
    //     }
    //     _ => {
    //         unsafe {
    //             core::arch::asm!("mov rax, 0xff");
    //         }
    //     }
    // }
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

#[expect(clippy::upper_case_acronyms)]
#[repr(C, packed)]
pub struct RTC {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
}

#[repr(C, packed)]
pub struct TcpPacketRequest {
    pub header: tcp::TcpHeader,
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
    pub length: u16,
}
