use core::{arch::naked_asm, ptr::copy_nonoverlapping};

use x86_64::structures::idt::InterruptStackFrame;

use crate::{
    fs::{
        block::BlockDevice,
        fat12::{block::Floppy, check, fs::{fat83, Filesystem}},
        iso9660::Iso9660,
        vfs,
    },
    init::config::SYSTEM_CONFIG,
    input::{elf, irq},
    mem::uheap,
    net::{icmp, ipv4, serial, tcp},
    task::{
        queue::Message,
        scheduler::{self},
    },
    time::rtc,
};

const USERLAND_START: u64 = 0x600_000;
// Must match USERLAND_END in src/input/elf.rs.  Stacks are placed at
// 0x840_000–0x8C0_000, so any pointer from a user stack lives above the
// old 0x800_000 ceiling and was incorrectly rejected by every syscall.
const USERLAND_END: u64 = 0xA00_000;

#[repr(u64)]
enum SyscallReturnCode {
    Ok = 0x00,
    NotImplemented = 0xfb,
    InvalidInput = 0xfc,
    FilesystemError = 0xfd,
    FileNotFound = 0xfe,
    InvalidSyscall = 0xff,
}

static mut MSG_BUF: [[u8; 512]; 10] = [
    [0; 512], [0; 512], [0; 512], [0; 512], [0; 512], [0; 512], [0; 512], [0; 512], [0; 512],
    [0; 512],
];

/// This function is the syscall ABI dispatching routine. It is called exclusively from the ISR
/// for interrupt 0x7f.
#[unsafe(naked)]
pub extern "x86-interrupt" fn syscall_handler(_: InterruptStackFrame) -> ! {
    naked_asm!(
        "mov rcx, rdx",
        "cld",

        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        //"push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        "sub rsp, 8",
        "mov rdx, rax",

        "call {syscall_inner}",

        "mov r9, rax",
        "add rsp, 8",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        //"pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",

        "mov rax, r9",

        "iretq",
        syscall_inner = sym syscall_inner,
    );
}

extern "C" fn syscall_inner(arg1: u64, arg2: u64, syscall_no: u64) -> u64 {
    // Re-enable interrupts so the PIT timer can preempt long-running syscalls.
    // The scheduler uses try_lock, so a timer tick during a scheduler operation
    // will simply fail to acquire the lock and return the old RSP unchanged.
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
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
         *  Arg1: any
         *  Arg2: program return code
         */
        0x00 => {
            unsafe {
                let pid = scheduler::get_current_pid();

                rprint!("[TASK ");
                //rprintn!(arg1);
                rprintn!(pid);
                rprint!("]: exit, return code: ");
                rprintn!(arg2);
                rprint!("\n");

                scheduler::kill(pid);
                scheduler::wake(scheduler::get_shell_pid());

                core::arch::asm!("sti");
                loop {
                    core::arch::asm!("int 0x20");
                    core::arch::asm!("hlt");
                }
            };
        }

        /*
         *  Syscall 0x01 --- Get/Set system info
         *
         *  Arg1: 0x01 or 0x02
         *  Arg2: pointer to system info struct (*mut SysInfo)
         */
        0x01 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let sysinfo_ptr = arg2 as *mut SysInfo;

            match arg1 {
                0x01 => unsafe {
                    if let Some(sc) = SYSTEM_CONFIG.try_lock() {
                        let name = sc.get_host();
                        let user = sc.get_user();
                        let version = sc.get_version();
                        let path = sc.get_path();
                        let path_cluster = sc.get_path_cluster() as u32;

                        if let Some(nm) = (*sysinfo_ptr).system_name.get_mut(0..name.len()) {
                            nm.copy_from_slice(name);
                        }

                        if let Some(us) = (*sysinfo_ptr).system_user.get_mut(0..user.len()) {
                            us.copy_from_slice(user);
                        }

                        if let Some(ph) = (*sysinfo_ptr).system_path.get_mut(0..path.len()) {
                            ph.copy_from_slice(path);
                        }

                        (*sysinfo_ptr).system_path_cluster = path_cluster;

                        if let Some(vn) = (*sysinfo_ptr).system_version.get_mut(0..version.len()) {
                            vn.copy_from_slice(version);
                        }

                        (*sysinfo_ptr).system_uptime =
                            crate::time::acpi::get_uptime_seconds() as u32;
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
                return SyscallReturnCode::InvalidInput as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
            }

            match arg1 {
                0x01 => {
                    irq::pipe_subscribe(arg2);
                }

                0x02 => {
                    irq::pipe_unsubscribe(arg2);
                }

                0x03 => {
                    let pid = unsafe { scheduler::get_current_pid() };

                    unsafe {
                        #[expect(static_mut_refs)]
                        for s in irq::RECEPTORS.iter() {
                            if s.pid == pid {
                                s.copy_to_user(arg2 as *mut u8, 16);
                                break;
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        /*
         *  Syscall 0x04 --- Get millisecond tick count
         *
         *  Arg1: 0x00 (unused)
         *  Arg2: 0x00 (unused)
         *  Returns: elapsed milliseconds since boot (10 ms resolution at 100 Hz PIT)
         */
        0x04 => {
            return crate::time::acpi::get_tick_count() * 10;
        }

        /*
         *  Syscall 0x05 --- Sleep for N milliseconds
         *
         *  Arg1: duration in milliseconds (rounded up to the next 10 ms PIT tick)
         *  Arg2: 0x00 (unused)
         *
         *  Marks the calling process as Blocked until the requested ticks have elapsed.
         *  The scheduler checks sleep_until on every PIT interrupt and wakes the process
         *  automatically — no busy-wait in the kernel.
         */
        0x05 => {
            if arg1 > 0 {
                let ticks = (arg1 + 9) / 10; // ceil(ms / 10)
                let wake_tick = crate::time::acpi::get_tick_count() + ticks;
                unsafe {
                    scheduler::sleep_current(wake_tick);
                }
            }
        }

        /*
         *  Syscall 0x0a --- Allocate memory from the userland heap
         *
         *  Arg1: size in bytes to allocate
         *  Arg2: unused (0x00)
         *  Returns: virtual address of the allocated block (in 0xC00_000–0xFFF_FFF),
         *           or 0x00 on failure.  The block is zeroed.
         */
        0x0a => {
            return uheap::malloc(arg1 as usize);
        }

        /*
         *  Syscall 0x0b --- Reallocate a heap block
         *
         *  Arg1: pointer to the existing block (or 0 to allocate fresh)
         *  Arg2: new size in bytes (0 frees the block and returns 0)
         *  Returns: virtual address of the (possibly moved) block, or 0 on failure.
         */
        0x0b => {
            return uheap::realloc(arg1, arg2 as usize);
        }

        /*
         *  Syscall 0x0f --- Free a heap block
         *
         *  Arg1: pointer to the block to free (must be in 0xC00_000–0xFFF_FFF)
         *  Arg2: 0x00
         */
        0x0f => {
            uheap::free(arg1);
        }

        /*
         *  Syscall 0x10 --- Print data to standard output
         *
         *  Arg1: pointer to data (&[u8])
         *  Arg2: length in bytes to print
         */
        0x10 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
            }

            clear_screen!();
        }

        /*
         *  Syscall 0x12 --- Write graphical pixel
         *
         *  Arg1: (x << 16) | y  — screen coordinates
         *  Arg2: 0x00RRGGBB color
         */
        0x12 => unsafe {
            let x = (arg1 as u32 >> 16) as u32;
            let y = (arg1 as u32 & 0xffff) as u32;
            let color = arg2 as u32;

            let fb = crate::init::check::FRAMEBUFFER_PTR;
            if fb.addr != 0 && x < fb.width && y < fb.height {
                let ptr = fb.addr as *mut u32;
                let offset = y * (fb.pitch / 4) + x;
                ptr.add(offset as usize).write_volatile(color);
            }
        },

        /*
         *  Syscall 0x13 --- Render VGA mode 13h framebuffer (320×200, 256-color)
         *
         *  Arg1: userland pointer to 64000-byte palette-indexed buffer
         *  Arg2: userland pointer to 768-byte palette (256 × RGB triplets), or 0 for default
         */
        0x13 => unsafe {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let fb = crate::init::check::FRAMEBUFFER_PTR;
            if fb.addr == 0 {
                return SyscallReturnCode::Ok as u64;
            }

            let vga_buf = arg1 as *const u8;
            let fb_ptr = fb.addr as *mut u32;
            let pitch_px = fb.pitch / 4;

            /* Use caller-supplied palette if valid, else fall back to default VGA palette */
            let use_custom = (USERLAND_START..=USERLAND_END).contains(&arg2);
            let pal_ptr = if use_custom {
                arg2 as *const u8
            } else {
                core::ptr::null()
            };

            for y in 0..200u32 {
                for x in 0..320u32 {
                    let idx = *vga_buf.add((y * 320 + x) as usize) as usize;
                    let color: u32 = if use_custom && !pal_ptr.is_null() {
                        let r = *pal_ptr.add(idx * 3) as u32;
                        let g = *pal_ptr.add(idx * 3 + 1) as u32;
                        let b = *pal_ptr.add(idx * 3 + 2) as u32;
                        (r << 16) | (g << 8) | b
                    } else {
                        vga_default_color(idx as u8)
                    };
                    let offset = y * pitch_px + x;
                    fb_ptr.add(offset as usize).write_volatile(color);
                }
            }
        },

        /*
         *  Syscall 0x14 --- Map VGA graphics RAM into the calling process
         *
         *  Arg1: 0x00 (reserved)
         *  Arg2: pointer to u64 (*mut u64) — receives the virtual base address
         *
         *  Maps physical 0xA0000–0xAFFFF (64 KiB EGA/VGA window) at virtual
         *  0xA00_000 in the current process's page table with USER+WRITE.
         *  Idempotent: safe to call more than once.
         *  On success the virtual base (0xA00_000) is written to *arg2.
         */
        0x14 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let virt_base = unsafe { crate::mem::pages::map_vram() };
            if virt_base == 0 {
                return SyscallReturnCode::InvalidInput as u64;
            }

            unsafe {
                *(arg2 as *mut u64) = virt_base;
            }
        }

        /*
         *  Syscall 0x15 --- Set VGA hardware video mode
         *
         *  Arg1: mode number
         *    0x03 — 80×25 color text  (restores kernel shell display)
         *    0x0D — 320×200 16-color planar (EGA-style; VRAM at 0xA0000)
         *    0x12 — 640×480 16-color planar (VRAM at 0xA0000, 4 planes)
         *    0x13 — 320×200 256-color unchained (VRAM at 0xA0000, linear)
         *  Arg2: 0x00 (reserved)
         *
         *  Programs Sequencer, CRTC, GC and AC registers directly.
         *  After a successful call, graphical modes use the VGA window
         *  mapped by syscall 0x14 (MAP_VRAM).  Mode 0x03 restores the
         *  VGA text buffer at 0xB8000 as the active display.
         */
        0x15 => {
            let mode = arg1 as u8;
            let ok = unsafe { crate::video::vga_hw::set_video_mode(mode) };

            if !ok {
                return SyscallReturnCode::InvalidInput as u64;
            }

            // Keep the kernel's VIDEO_MODE consistent: text mode 0x03 reverts
            // to the VGA text buffer; all others leave the VESA framebuffer
            // address intact (it is simply not displayed by the hardware).
            if mode == 0x03 {
                crate::video::mode::set_mode_text();
            }
        }

        /*
         *  Syscall 0x1a --- Play a frequency
         *
         *  Arg2: frequency in Hz
         *  Arg2: duration in ms
         */
        0x1a => {
            if !(20..=20_000).contains(&arg1) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            crate::audio::beep::beep(arg1 as u32);
            crate::audio::midi::wait_millis(arg2 as u16);
            crate::audio::beep::stop_beep();
        }

        /*
         *  Syscall 0x1b --- Play an audio file
         *
         *  Arg1: audio file type (0x01 = MIDI format 0)
         *  Arg2: pointer to NUL-terminated file name (*const u8)
         */
        0x1b => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let name_slice = unsafe { nul_terminated_slice(arg2 as *const u8, 64) };
            let (rel, base) = vfs_resolve_fat12(name_slice);
            let name83 = fat83(rel);
            let mut buf: [u8; 4096] = [0u8; 4096];

            match arg1 {
                0x01 => {
                    let floppy = Floppy::init();
                    match Filesystem::new(&floppy) {
                        Ok(fs) => match fs.find_entry(base, &name83) {
                            None => return SyscallReturnCode::FileNotFound as u64,
                            Some(entry) => { fs.read_file(entry.start_cluster, &mut buf); }
                        },
                        Err(e) => { rprint!(e); rprint!("\n");
                            return SyscallReturnCode::FilesystemError as u64; }
                    }
                    if let Some(midi) = crate::audio::midi::parse_midi_format0(&buf) {
                        crate::audio::midi::play_midi(&midi);
                        crate::audio::beep::stop_beep();
                    } else {
                        return SyscallReturnCode::FilesystemError as u64;
                    }
                }
                _ => { return SyscallReturnCode::InvalidInput as u64; }
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
                return SyscallReturnCode::InvalidInput as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
            }

            let name_slice = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };

            // ISO9660 dispatch: absolute paths that resolve to an iso9660 mount.
            if let Some(iso_rel) = vfs::try_iso9660_absolute(name_slice) {
                match Iso9660::probe() {
                    None => return SyscallReturnCode::FilesystemError as u64,
                    Some(iso) => match iso.resolve(iso_rel) {
                        None => return SyscallReturnCode::FileNotFound as u64,
                        Some(e) if e.is_dir => return SyscallReturnCode::InvalidInput as u64,
                        Some(e) => {
                            let buf_ptr = arg2 as *mut u8;
                            let buf = unsafe { core::slice::from_raw_parts_mut(buf_ptr, e.size as usize) };
                            iso.read_file(&e, buf);
                            return SyscallReturnCode::Ok as u64;
                        }
                    },
                }
            }

            let (rel, base) = vfs_resolve_fat12(name_slice);
            let name83 = fat83(rel);
            let buf_ptr = arg2 as *mut u8;
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    let entry = match fs.find_entry(base, &name83) {
                        Some(e) if e.attr & 0x10 == 0 => e,
                        _ => return SyscallReturnCode::FileNotFound as u64,
                    };
                    let mut cluster = entry.start_cluster;
                    let mut offset = 0u32;
                    while offset < entry.file_size {
                        let lba = fs.cluster_to_lba(cluster);
                        let mut sector = [0u8; 512];
                        fs.device.read_sector(lba, &mut sector);
                        unsafe {
                            copy_nonoverlapping(sector.as_ptr(), buf_ptr.add(offset as usize), 512);
                        }
                        cluster = fs.read_fat12_entry(cluster);
                        if cluster >= 0xFF8 || cluster == 0 { break; }
                        offset += 512;
                    }
                }
                Err(e) => {
                    rprint!(e); rprint!("\n");
                    return SyscallReturnCode::FilesystemError as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
            }

            let name_slice = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };
            if vfs::try_iso9660_absolute(name_slice).is_some() {
                return SyscallReturnCode::InvalidInput as u64;
            }
            let (rel, base) = vfs_resolve_fat12(name_slice);
            let name83 = fat83(rel);
            let buf_ptr = arg2 as *const [u8; 512];
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => unsafe { fs.write_file(base, &name83, &*buf_ptr); },
                Err(e) => {
                    rprint!(e); rprint!("\n");
                    return SyscallReturnCode::FilesystemError as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
            }

            let old_slice = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };
            let new_slice = unsafe { nul_terminated_slice(arg2 as *const u8, 64) };
            if vfs::try_iso9660_absolute(old_slice).is_some() {
                return SyscallReturnCode::InvalidInput as u64;
            }
            let (rel_old, base) = vfs_resolve_fat12(old_slice);
            let old83 = fat83(rel_old);
            let new83 = fat83(new_slice);
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    if fs.find_entry(base, &old83).is_none() {
                        return SyscallReturnCode::FileNotFound as u64;
                    }
                    fs.rename_file(base, &old83, &new83);
                }
                Err(e) => { rprint!(e); rprint!("\n");
                    return SyscallReturnCode::FilesystemError as u64; }
            }
        }

        /*
         *  Syscall 0x23 --- Delete a directory entry
         *
         *  Arg1: pointer to filename
         *  Arg2: 0x00
         */
        0x23 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) || arg2 != 0 {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let name_slice = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };
            if vfs::try_iso9660_absolute(name_slice).is_some() {
                return SyscallReturnCode::InvalidInput as u64;
            }
            let (rel, base) = vfs_resolve_fat12(name_slice);
            let name83 = fat83(rel);
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    if fs.find_entry(base, &name83).is_none() {
                        return SyscallReturnCode::FileNotFound as u64;
                    }
                    fs.delete_file(base, &name83);
                }
                Err(e) => { rprint!(e); rprint!("\n");
                    return SyscallReturnCode::FilesystemError as u64; }
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
            return SyscallReturnCode::NotImplemented as u64;
        }

        /*
         *  Syscall 0x25 --- Write to the FAT table
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to value (*const u84)
         */
        0x25 => {
            // TODO
            return SyscallReturnCode::NotImplemented as u64;
        }

        /*
         *  Syscall 0x26 --- Insert entry into cluster
         *
         *  Arg1: cluster No.
         *  Arg2: pointer to a new directory entry (*const Entry)
         */
        0x26 => {
            // TODO
            return SyscallReturnCode::NotImplemented as u64;
        }

        /*
         *  Syscall 0x27 --- Add new subdirectory
         *
         *  Arg1: pointer to parent directory absolute path (*const u8)
         *  Arg2: pointer to new subdirectory name (*const u8)
         */
        0x27 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1)
                || !(USERLAND_START..=USERLAND_END).contains(&arg2)
            {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let parent_slice = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };
            if vfs::try_iso9660_absolute(parent_slice).is_some() {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let name_ptr = arg2 as *const u8;
            let (name, ext) = format_filename(name_ptr);
            let mut filename: [u8; 11] = [b' '; 11];
            filename[0..8].copy_from_slice(&name);
            filename[8..11].copy_from_slice(&ext);

            let (rel, base) = vfs_resolve_fat12(parent_slice);
            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    let parent_cluster: u16 = if rel.is_empty() {
                        base
                    } else {
                        match fs.resolve_path_from(base, rel) {
                            None => return SyscallReturnCode::FileNotFound as u64,
                            Some(e) if e.attr & 0x10 == 0 => return SyscallReturnCode::InvalidInput as u64,
                            Some(e) => e.start_cluster,
                        }
                    };
                    fs.create_subdirectory(&filename, parent_cluster);
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");
                    return SyscallReturnCode::FilesystemError as u64;
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
                        core::ptr::copy_nonoverlapping(kentries.as_ptr(), entries, 32);
                    }
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    return SyscallReturnCode::FilesystemError as u64;
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
            return SyscallReturnCode::NotImplemented as u64;
        }

        /*
         *  Syscall 0x2A --- Load and run ELF executable in background
         *
         *  Arg1: file name (NUL-terminated, max 12 chars)
         *  Arg2: args string (space-delimited; first token = argv[0]; 0 = use name only)
         *  Returns: PID on success, 0 on failure
         */
        0x2A => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let name_slice = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };

            // arg2: optional full args string matching push_user_args convention.
            // If absent or out of range, use the name as the sole argv[0] token.
            let args_slice: &[u8] = if arg2 != 0 && (USERLAND_START..=USERLAND_END).contains(&arg2) {
                unsafe { nul_terminated_slice(arg2 as *const u8, 128) }
            } else {
                name_slice
            };

            let pid = elf::run_elf(name_slice, args_slice, elf::RunMode::Background);
            return pid as u64;
        }

        /*
         *  Syscall 0x2B --- Run filesystem check (fsck)
         *
         *  Arg1: unused
         *  Arg2: pointer to FsckReport_T (4 × u64: errors, orphans, cross_linked, invalid)
         */
        0x2B => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let report = check::run_check();
            let out = arg2 as *mut u64;

            unsafe {
                out.add(0).write_volatile(report.errors as u64);
                out.add(1).write_volatile(report.orphan_clusters as u64);
                out.add(2).write_volatile(report.cross_linked as u64);
                out.add(3).write_volatile(report.invalid_entries as u64);
            }
        }

        /*
         *  Syscall 0x2C --- List VFS mount table
         *
         *  Arg1: unused
         *  Arg2: pointer to buffer (MAX_MOUNTS × 34 bytes: 32-byte path, 1-byte path_len, 1-byte fs_type)
         *        fs_type encoding: 0=none, 1=rootfs, 2=fat12
         *  Returns: number of mounts written
         */
        0x2C => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let buf = arg2 as *mut u8;
            let mut count = 0u64;

            if let Some(vfs_table) = vfs::VFS.try_lock() {
                let n = vfs_table.count();
                for i in 0..n {
                    if let Some(m) = vfs_table.get(i) {
                        let fs_type_u8: u8 = match m.fs_type {
                            vfs::FsType::None    => 0,
                            vfs::FsType::Root    => 1,
                            vfs::FsType::Fat12   => 2,
                            vfs::FsType::Iso9660 => 3,
                        };
                        unsafe {
                            let entry = buf.add(i * 34);
                            copy_nonoverlapping(m.path.as_ptr(), entry, 32);
                            entry.add(32).write_volatile(m.path_len as u8);
                            entry.add(33).write_volatile(fs_type_u8);
                        }
                        count += 1;
                    }
                }
            }

            return count;
        }

        /*
         *  Syscall 0x2D --- List directory by path (VFS-aware: FAT12 + ISO9660)
         *
         *  Arg1: pointer to NUL-terminated absolute path string (*const u8)
         *  Arg2: pointer to output buffer (up to 32 × 38-byte VfsDirEntry records)
         *        Layout per entry: name[32], name_len: u8, is_dir: u8, size: u32 (LE)
         *  Returns: entry count (0–32) on success; u64::MAX (-1 as int64_t) on any error.
         *  Callers MUST treat any return value outside [0, 32] as an error.
         */
        0x2D => {
            // u64::MAX reads as -1 in C's int64_t — unambiguously not a valid count.
            const ERR: u64 = u64::MAX;

            if !(USERLAND_START..=USERLAND_END).contains(&arg1)
                || !(USERLAND_START..=USERLAND_END).contains(&arg2)
            {
                return ERR;
            }

            let path = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };
            let buf  = arg2 as *mut u8;

            // ISO9660 branch.
            if let Some(iso_rel) = vfs::try_iso9660_absolute(path) {
                match Iso9660::probe() {
                    None => return ERR,
                    Some(iso) => {
                        let dir = if iso_rel.is_empty() {
                            crate::fs::iso9660::IsoEntry {
                                is_dir: true, lba: iso.root_lba, size: iso.root_size,
                                ..Default::default()
                            }
                        } else {
                            match iso.resolve(iso_rel) {
                                None               => return ERR,
                                Some(e) if !e.is_dir => return ERR,
                                Some(e)            => e,
                            }
                        };
                        let mut entries = [crate::fs::iso9660::IsoEntry::default(); 32];
                        let count = iso.list_dir(dir.lba, dir.size, &mut entries);
                        for (i, e) in entries[..count].iter().enumerate() {
                            unsafe {
                                let out = buf.add(i * 38);
                                copy_nonoverlapping(e.name.as_ptr(), out, 32);
                                out.add(32).write_volatile(e.name_len);
                                out.add(33).write_volatile(e.is_dir as u8);
                                copy_nonoverlapping(e.size.to_le_bytes().as_ptr(), out.add(34), 4);
                            }
                        }
                        return count as u64;
                    }
                }
            }

            // FAT12 branch.
            let (rel, base) = vfs_resolve_fat12(path);
            let floppy = Floppy::init();
            match Filesystem::new(&floppy) {
                Err(_) => return ERR,
                Ok(fs) => {
                    let dir_cluster: u16 = if rel.is_empty() {
                        base
                    } else {
                        match fs.resolve_path_from(base, rel) {
                            None                           => return ERR,
                            Some(e) if e.attr & 0x10 == 0 => return ERR,
                            Some(e)                        => e.start_cluster,
                        }
                    };

                    // Collect FAT12 entries into a local array first.
                    let mut fat_entries = [crate::fs::fat12::entry::Entry::default(); 32];
                    let mut kcount = 0usize;
                    fs.for_each_entry(dir_cluster, |entry| {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 { return; }
                        if entry.attr & 0x08 != 0 { return; } // volume label
                        if kcount < 32 { fat_entries[kcount] = *entry; kcount += 1; }
                    });

                    for (i, entry) in fat_entries[..kcount].iter().enumerate() {
                        unsafe {
                            let out = buf.add(i * 38);
                            let mut name_buf = [0u8; 32];
                            let mut name_len = 0usize;
                            // base name (trim trailing spaces)
                            for j in 0..8usize {
                                if entry.name[j] != b' ' { name_buf[name_len] = entry.name[j]; name_len += 1; }
                            }
                            // extension (files only, trim spaces)
                            if entry.attr & 0x10 == 0 && entry.ext[0] != b' ' {
                                name_buf[name_len] = b'.'; name_len += 1;
                                for j in 0..3usize {
                                    if entry.ext[j] != b' ' { name_buf[name_len] = entry.ext[j]; name_len += 1; }
                                }
                            }
                            copy_nonoverlapping(name_buf.as_ptr(), out, 32);
                            out.add(32).write_volatile(name_len as u8);
                            out.add(33).write_volatile(if entry.attr & 0x10 != 0 { 1u8 } else { 0u8 });
                            copy_nonoverlapping(entry.file_size.to_le_bytes().as_ptr(), out.add(34), 4);
                        }
                    }
                    return kcount as u64;
                }
            }
        }

        /*
         *  Syscall 0x2E --- Change working directory (chdir)
         *
         *  Arg1: pointer to NUL-terminated absolute path (*const u8)
         *  Arg2: unused (0x00)
         *
         *  Verifies the path is an existing directory, then updates SYSTEM_CONFIG
         *  with the new path string and FAT12 cluster (0 for ISO9660 or FAT12 root).
         */
        0x2E => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let path = unsafe { nul_terminated_slice(arg1 as *const u8, 64) };

            // ISO9660: verify directory exists, store path with cluster=0
            if let Some(iso_rel) = vfs::try_iso9660_absolute(path) {
                match Iso9660::probe() {
                    None => return SyscallReturnCode::FilesystemError as u64,
                    Some(iso) => {
                        let ok = if iso_rel.is_empty() {
                            true
                        } else {
                            match iso.resolve(iso_rel) {
                                Some(e) if e.is_dir => true,
                                _ => false,
                            }
                        };
                        if !ok {
                            return SyscallReturnCode::FileNotFound as u64;
                        }
                    }
                }
                if let Some(mut c) = SYSTEM_CONFIG.try_lock() {
                    c.set_path(path, 0);
                }
                return SyscallReturnCode::Ok as u64;
            }

            // FAT12 (absolute /mnt/fat/... or relative to cwd)
            let (rel, base) = vfs_resolve_fat12(path);
            let floppy = Floppy::init();
            match Filesystem::new(&floppy) {
                Err(e) => {
                    rprint!(e); rprint!("\n");
                    return SyscallReturnCode::FilesystemError as u64;
                }
                Ok(fs) => {
                    let cluster: u16 = if rel.is_empty() {
                        base
                    } else {
                        match fs.resolve_path_from(base, rel) {
                            None => return SyscallReturnCode::FileNotFound as u64,
                            Some(e) if e.attr & 0x10 == 0 => return SyscallReturnCode::InvalidInput as u64,
                            Some(e) => e.start_cluster,
                        }
                    };
                    if let Some(mut c) = SYSTEM_CONFIG.try_lock() {
                        c.set_path(path, cluster);
                    }
                }
            }
        }

        /*
         *  Syscall 0x2F --- List scheduler tasks
         *
         *  Arg1: pointer to output buffer (max 10 × 20-byte TaskInfo entries)
         *  Arg2: max entries to write (capped at 10)
         *  Returns: number of entries written
         */
        0x2F => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg1) {
                return SyscallReturnCode::InvalidInput as u64;
            }
            let max = if arg2 > 0 && (arg2 as usize) <= 10 { arg2 as usize } else { 10 };
            let count = scheduler::list_tasks(arg1 as *mut u8, max);
            return count as u64;
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

            // VGA I/O registers are byte-wide; a 32-bit outd would cause QEMU to
            // decompose the write into 4 consecutive byte writes (port, port+1, ...),
            // corrupting adjacent registers (e.g. 0x3C8→idx also hits 0x3C9 with 0).
            unsafe {
                crate::input::port::write_u8(*port, *value as u8);
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
                return SyscallReturnCode::InvalidInput as u64;
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
                        return SyscallReturnCode::InvalidInput as u64;
                    }

                    serial::init();
                }

                // Read from UART — returns InvalidInput when no byte is ready,
                // so userland can distinguish "no data" from a real read.
                0x02 => {
                    if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                        return SyscallReturnCode::InvalidInput as u64;
                    }

                    if !serial::ready() {
                        return SyscallReturnCode::InvalidInput as u64;
                    }

                    let value = arg2 as *mut u32;

                    unsafe {
                        *value = serial::read() as u32;
                    }
                }

                // Write to UART
                0x03 => {
                    if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                        return SyscallReturnCode::InvalidInput as u64;
                    }

                    let value = arg2 as *const u32;

                    unsafe {
                        serial::write(*value as u8);
                    }
                }

                _ => {
                    return SyscallReturnCode::InvalidInput as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
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
                            return SyscallReturnCode::InvalidInput as u64;
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
                            return SyscallReturnCode::InvalidInput as u64;
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
                return SyscallReturnCode::InvalidInput as u64;
            }

            if arg1 == 0x01 {
                let packet = arg2 as *const u8;

                let header = packet as *const ipv4::Ipv4Header;

                unsafe {
                    let total_len = u16::from_be((*header).total_length);
                    let slice = core::slice::from_raw_parts(packet, total_len as usize);

                    ipv4::send_packet(slice);
                }
            } else if arg1 == 0x04 {
                // Raw Ethernet frame TX: derive the frame length from the headers.
                // ETH header = 14 bytes; ethertype at bytes [12..14].
                let packet = arg2 as *const u8;
                unsafe {
                    let ethertype = u16::from_be_bytes([*packet.add(12), *packet.add(13)]);
                    let len: usize = match ethertype {
                        0x0800 => {
                            // IPv4: total_length is at bytes [16..18] of the full frame
                            let ip_total = u16::from_be_bytes([*packet.add(16), *packet.add(17)]);
                            14 + ip_total as usize
                        }
                        0x0806 => 14 + 28, // ARP over Ethernet is always 42 bytes
                        _ => 0,
                    };
                    if len >= 14 && len <= 1514 {
                        let slice = core::slice::from_raw_parts(packet, len);
                        let _ = crate::net::rtl8139::send_frame(slice, len);
                    }
                }
            }
        }

        /*
         *  Syscall 0x35 --- Receive data. Try to pop the port queue. Blocking op
         *
         *  Arg1: target pid
         *  Arg2: pointer to a buffer
         */
        0x35 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            unsafe {
                let buf = arg2 as *mut u8;
                let current_pid = scheduler::get_current_pid();

                if let Some(msg) = scheduler::pop_msg(current_pid) {
                    let len = if msg.port_id > 0 { msg.port_id } else { 512 };
                    copy_nonoverlapping(msg.buf_addr as *const u8, buf, len);
                    return len as u64;
                } else {
                    scheduler::block(current_pid, Message::new(0, 0xff, current_pid, 512));
                }
            }
        }

        /*
         *  Syscall 0x36 --- Send data. Try to push to the port queue
         *
         *  Arg1: op code
         *  Arg2: pointer to a buffer
         */
        0x36 => {
            if !(USERLAND_START..=USERLAND_END).contains(&arg2) {
                return SyscallReturnCode::InvalidInput as u64;
            }

            let target_pid = arg1 as usize;
            let buf = arg2 as *const u8;

            unsafe {
                copy_nonoverlapping(buf, MSG_BUF[0].as_mut_ptr(), 512);
                let current_pid = scheduler::get_current_pid();

                let msg = Message::new(0, current_pid, target_pid, 512);
                scheduler::push_msg(target_pid, msg);
                scheduler::wake(target_pid);
            }
        }

        /*
         *  Syscall 0x37 --- Ethernet driver registration / port binding.
         *
         *  Arg1 = 0   → register as the global Ethernet driver (handles ARP, ICMP, and
         *               any TCP port not explicitly bound). Initialises the RTL8139.
         *  Arg1 = N>0 → bind TCP destination port N to the calling process. The kernel
         *               will deliver only frames whose TCP dest port matches N.
         */
        0x37 => unsafe {
            let pid = scheduler::get_current_pid();
            let port = arg1 as u16;
            if port == 0 {
                crate::net::netdrv::register_driver(pid);
            } else {
                crate::net::netdrv::bind_port(port, pid);
            }
        },

        /*
         *  Unknown syscall
         */
        _ => {
            rprint!("Unknown syscall: ");
            rprintn!(syscall_no);
            rprint!("\n");

            return SyscallReturnCode::InvalidSyscall as u64;
        }
    }

    SyscallReturnCode::Ok as u64
}

/// Read a NUL-terminated string from a validated userland pointer into a slice.
unsafe fn nul_terminated_slice(ptr: *const u8, max: usize) -> &'static [u8] {
    let len = (0..max).take_while(|&i| *ptr.add(i) != 0).count();
    core::slice::from_raw_parts(ptr, len)
}

/// Resolve a user-provided filename/path to (fat12_relative_slice, base_cluster).
/// Absolute paths under /mnt/fat are stripped to their FAT12-relative tail;
/// bare names are resolved relative to the current working directory.
fn vfs_resolve_fat12(path: &[u8]) -> (&[u8], u16) {
    if let Some(rel) = vfs::try_fat12_absolute(path) {
        return (rel, 0); // start from FAT12 root
    }
    let cwd = SYSTEM_CONFIG.try_lock().map_or(0, |c| c.get_path_cluster());
    (path, cwd)
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

/// Convert a VGA 256-color palette index to 0x00RRGGBB using the standard BIOS default palette.
fn vga_default_color(idx: u8) -> u32 {
    /* First 16 entries: standard CGA/EGA colors */
    const CGA: [u32; 16] = [
        0x000000, 0x0000AA, 0x00AA00, 0x00AAAA, 0xAA0000, 0xAA00AA, 0xAA5500, 0xAAAAAA, 0x555555,
        0x5555FF, 0x55FF55, 0x55FFFF, 0xFF5555, 0xFF55FF, 0xFFFF55, 0xFFFFFF,
    ];
    if (idx as usize) < 16 {
        return CGA[idx as usize];
    }
    /* 16-231: 6×6×6 color cube */
    if idx < 232 {
        let v = idx - 16;
        let b = (v % 6) * 51;
        let g = ((v / 6) % 6) * 51;
        let r = (v / 36) * 51;
        return ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    }
    /* 232-255: grayscale */
    let l = (idx - 232) as u32 * 10 + 8;
    (l << 16) | (l << 8) | l
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
    pub system_path_cluster: u32,
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
