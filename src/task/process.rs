use crate::input::keyboard::keyboard_loop;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    // iretq stack frame
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Kernel, // RING0
    Driver, // RING1
    PrUser, // RING2
    User,   // RING3
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Ready,
    Running,
    Idle,
    Crashed,
    Dead,
}

#[derive(Debug, Clone, Copy)]
pub struct Process {
    id: usize,
    name: [u8; 16],
    mode: Mode,
    pub status: Status,
    context: Context,
    kernel_stack: [u8; 8096],
}

extern "C" {
    static mut tss64: crate::init::idt::Tss64;
}

pub static mut INIT_DONE: bool = false;

static mut PROCESS_LIST: [Option<Process>; 4] = [None, None, None, None];
pub static mut CURRENT_PID: usize = 0;
static mut NEXT_FREE_PID: usize = 0;

#[no_mangle]
pub unsafe fn schedule(old: *mut Context) -> *mut Context {
    if !INIT_DONE {
        crate::input::port::write(0x20, 0x20);
        return old;
    }

    let mut next = (CURRENT_PID + 1) % PROCESS_LIST.len();

    loop {
        if !PROCESS_LIST[next].is_none() && PROCESS_LIST[next].unwrap().status == Status::Dead {
            PROCESS_LIST[next] = None;
        }

        if !PROCESS_LIST[next].is_none()
            && PROCESS_LIST[next].unwrap().status != Status::Idle
            && PROCESS_LIST[next].unwrap().status != Status::Crashed
        {
            break;
        }
        next += 1;
        next %= PROCESS_LIST.len();
    }

    PROCESS_LIST[CURRENT_PID].as_mut().unwrap().context = *old;

    if PROCESS_LIST[CURRENT_PID].unwrap().status != Status::Idle
        && PROCESS_LIST[CURRENT_PID].unwrap().status != Status::Crashed
        && PROCESS_LIST[CURRENT_PID].unwrap().status != Status::Dead
    {
        PROCESS_LIST[CURRENT_PID].as_mut().unwrap().status = Status::Ready;
    }

    if PROCESS_LIST[CURRENT_PID].unwrap().status == Status::Dead {
        PROCESS_LIST[CURRENT_PID] = None;
    }

    PROCESS_LIST[next].as_mut().unwrap().status = Status::Running;
    CURRENT_PID = next;

    let kstack = PROCESS_LIST[next].unwrap().kernel_stack;
    tss64.rsp0 = (kstack.as_ptr() as u64) + kstack.len() as u64;
    tss64.rsp0 &= !0xF;

    crate::input::port::write(0x20, 0x20);
    &mut PROCESS_LIST[next].as_mut().unwrap().context as *mut _
}

pub unsafe fn idle() {
    if !PROCESS_LIST[CURRENT_PID].is_none() {
        PROCESS_LIST[CURRENT_PID].as_mut().unwrap().status = Status::Idle;
    }
}

pub unsafe fn crash() {
    if !PROCESS_LIST[CURRENT_PID].is_none() {
        PROCESS_LIST[CURRENT_PID].as_mut().unwrap().status = Status::Crashed;
    }

    /*core::arch::asm!("int 0x20");
    loop {
        core::arch::asm!("hlt");
    }*/
}

pub unsafe fn kill(pid: usize) {
    if pid < PROCESS_LIST.len() && !PROCESS_LIST[pid].is_none() {
        rprint!("KILL PID ");
        rprintn!(pid);
        rprint!("\n\n");

        PROCESS_LIST[pid].as_mut().unwrap().status = Status::Dead;
    }
}

pub unsafe fn resume(pid: usize) {
    if pid < PROCESS_LIST.len() && !PROCESS_LIST[pid].is_none() {
        PROCESS_LIST[pid].as_mut().unwrap().status = Status::Ready;
    }
}

pub unsafe fn setup_processes() {
    let src = user_entry as *const u8;
    let dst = 0x7d0_000 as *mut u8;

    core::ptr::copy_nonoverlapping(src, dst, 4096);

    let proc0 = create_process(b"init", Mode::Kernel, 0, 0x190_000);
    let proc1 = create_process(b"clock", Mode::Kernel, 0x7d0_000, 0x7a0_000);
    let proc2 = create_process(b"shell", Mode::Kernel, keyboard_loop as u64, 0x700_000);

    PROCESS_LIST = [Some(proc0), Some(proc1), Some(proc2), None]
}

pub unsafe fn start_process(proc: Process) -> bool {
    for candidate in PROCESS_LIST.as_mut() {
        if candidate.is_none() || candidate.unwrap().status == Status::Dead {
            *candidate = Some(proc);
            return true;
        }
    }

    false
}

#[no_mangle]
extern "C" fn user_entry() -> ! {
    use crate::vga::buffer::Color;
    use crate::vga::write::{newline, number, string};

    //let mut i = 0;
    loop {
        unsafe {
            //i += 1;
            //rprintn!(i);
            //rprint!("\n");

            let vga_index: &mut isize = &mut 144;

            let (y, mo, d, h, m, s) = crate::time::rtc::read_rtc_full();

            // Hours
            number(vga_index, h as u64);
            string(vga_index, b":", crate::vga::buffer::Color::White);

            // Minutes
            if m < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, m as u64);
            string(vga_index, b":", Color::White);

            // Seconds
            if s < 10 {
                string(vga_index, b"0", Color::White);
            }
            number(vga_index, s as u64);
            newline(vga_index);

            for _ in 0..50_000 {
                //core::arch::asm!("mov rdx, 0", "int 0x7f", "hlt");
                core::arch::asm!("pause");
            }
        }
    }
}

pub unsafe fn list_processes() {
    println!("RUNNING PROCESSES");

    for process in PROCESS_LIST.iter().flatten() {
        printn!(process.id);
        print!("   ");
        printb!(&process.name);

        match process.status {
            Status::Ready => {
                print!(" (Ready)");
            }
            Status::Running => {
                print!(" (Running)");
            }
            Status::Idle => {
                print!(" (Idle)");
            }
            Status::Crashed => {
                print!(" (Crashed)");
            }
            Status::Dead => {
                print!(" (Dead)");
            }
        }

        println!();
    }
}

pub fn create_process(
    name_slice: &[u8],
    mode: Mode,
    entry_point: u64,
    process_stack_top: u64,
) -> Process {
    let mut name: [u8; 16] = [0; 16];

    if let Some(mut slice) = name.get_mut(0..name_slice.len()) {
        (*slice)[..name_slice.len()].copy_from_slice(name_slice);
    }

    let mut code_segment = 0;
    let mut stack_segment = 0;

    match mode {
        Mode::Kernel => {
            code_segment = 0x08;
            stack_segment = 0x10;
        }
        Mode::User => {
            code_segment = 0x1b;
            stack_segment = 0x23;
        }
        _ => {}
    }

    Process {
        id: unsafe {
            NEXT_FREE_PID += 1;
            NEXT_FREE_PID - 1
        },
        name,
        mode,
        status: Status::Ready,
        context: Context {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rdi: 0,
            rsi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            rip: entry_point,
            cs: code_segment,
            rflags: 0x202, // IF = 1
            rsp: process_stack_top,
            ss: stack_segment,
        },
        kernel_stack: [0; 8096],
    }
}
