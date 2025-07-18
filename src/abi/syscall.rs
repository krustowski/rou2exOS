use crate::{fs::fat12::{block::Floppy, fs::Filesystem}, input::elf::kernel_return, task::process::schedule};

/// This function is the syscall ABI dispatching routine. It is called exclusively from the ISR 
/// for interrupt 0x7f. 
#[no_mangle]
pub extern "C" fn syscall_handler() {
    let (syscall_no, arg1, arg2): (u64, u64, u64);
    let mut ret = 0;

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
        0x00 => {
            // PROCESS/TASK EXIT 
            rprint!("[TASK ");
            rprintn!(arg1);
            rprint!("]: bonjour\n");

            unsafe {
                core::arch::asm!(
                    "mov rdi, {0}",
                    "mov rsi, {1}",
                    //"jmp kernel_return",
                    //"call end_task",
                    "jmp kernel_return",
                    in(reg) arg1,
                    in(reg) arg2,
                );
            };
        }

        0x01 => {
            if arg2 < 0x600000 || arg1 > 0x800000 {
                ret = 0xfc;
                return;
            }

            let mut sysinfo_ptr = arg2 as *mut SysInfo;

            match arg1 {
                0x01 => {
                    unsafe {
                        let name = b"r2";
                        let user = b"guest";
                        let version = b"v0.8.3";

                        if let Some(nm) = (*sysinfo_ptr).system_name.get_mut(0..name.len()) {
                            nm.copy_from_slice(name);
                        }

                        if let Some(us) = (*sysinfo_ptr).system_user.get_mut(0..user.len()) {
                            us.copy_from_slice(user);
                        }

                        if let Some(vn) = (*sysinfo_ptr).system_version.get_mut(0..version.len()) {
                            vn.copy_from_slice(version);
                        }
                    }
                }
                0x02 => {
                    //
                }
                _ => {}
            }

            ret = 0;
        }

        0x10 => {
            if arg1 < 0x600000 || arg1 > 0x800000 {
                ret = 0xfc;
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

            ret = 0;
        }

        0x20 => {
            let name_ptr = arg1 as *const u8;

            let mut name = [b' '; 8];
            let mut ext = [b' '; 3];

            // TODO: Verify the pointer!

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
            }

            let buf_ptr = arg2 as *mut [u8; 512];

            use crate::fs::fat12::{block::Floppy, fs::Filesystem};

            let floppy = Floppy::init();

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(0, | entry | {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 || entry.attr & 0x10 != 0 {
                            ret = 0xfe;
                            return;
                        }

                        unsafe {
                            if !entry.name.starts_with(&name) || !entry.ext.starts_with(&ext) {
                                ret = 0xfc;
                                return
                            }

                            // Read the file directly into the client's buffer
                            fs.read_file(entry.start_cluster, &mut *buf_ptr);
                        }
                    });
                }
                Err(e) => {
                    rprint!(e);
                    rprint!("\n");

                    ret = 0xfd;
                }
            }

            ret = 0;
        }

        0x28 => {
            let path = arg1;
            let entries = arg2 as *mut [crate::fs::fat12::entry::Entry; 32];

            let floppy = Floppy::init();
            let mut offset = 0;

            match Filesystem::new(&floppy) {
                Ok(fs) => {
                    fs.for_each_entry(path as u16, |entry| {
                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 || entry.attr & 0x08 != 0x00 {
                            return;
                        }

                        unsafe {
                            if let Some(en) = (*entries).get_mut(offset) {
                                *en = *entry;
                                offset += 1;
                            }
                        }
                    });
                }
                Err(e) => {}
            }
        }

        0x30 => {
            let port = arg1 as *const u16;
            let value = arg2 as *const u32;

            unsafe {
                crate::input::port::write_u32(*port, *value);
            }

            ret = 0x00;
        }

        0x31 => {
            let port = arg1 as *const u16;
            let value = arg2 as *mut u32;

            unsafe {
                *value = crate::input::port::read_u32(*port);
            }

            ret = 0x00;
        }

        _ => {
            debug!("Unknown or non-implemented syscall: ");
            debugn!(syscall_no);
            debugln!("");

            ret = 0xff;
        }
    }

    unsafe {
        core::arch::asm!(
            "mov rax, {0:r}",
            in(reg) ret,
        );
    }
}

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
    pub system_version: [u8; 8],
    pub system_uptime: u32,
}

