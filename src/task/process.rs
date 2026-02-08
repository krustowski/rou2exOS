use crate::{input::keyboard::keyboard_loop, task::task::INIT_DONE};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Ready,
    Running,
    Idle,
    Dead,
}

#[derive(Debug, Clone, Copy)]
pub struct Process {
    id: u64,
    pub status: Status,
    context: Context,
    kernel_stack: [u8; 4096],
    //    next: *mut Process,
}

static mut PROCESS_LIST: [Option<Process>; 3] = [None, None, None];
static mut CURRENT_PID: usize = 0;

//static mut SCHEDULER_STARTED: bool = false;

#[no_mangle]
pub unsafe fn schedule(old: *mut Context) -> *mut Context {
    if !INIT_DONE {
        crate::input::port::write(0x20, 0x20);
        return old;
    }

    /*if !SCHEDULER_STARTED {
        SCHEDULER_STARTED = true;

        crate::input::port::write(0x20, 0x20);
        return &mut PROCESS_LIST[CURRENT_PID].as_mut().unwrap().context;
    }*/

    let mut next = (CURRENT_PID + 1) % PROCESS_LIST.len();

    if PROCESS_LIST[next].unwrap().status == Status::Idle {
        next += 1;
    }

    PROCESS_LIST[CURRENT_PID].as_mut().unwrap().context = *old;
    CURRENT_PID = next;

    crate::input::port::write(0x20, 0x20);
    &mut PROCESS_LIST[next].as_mut().unwrap().context as *mut _
}

/*pub unsafe fn schedule(context: *const Context) -> *const Context {
    #[expect(static_mut_refs)]
    let next_pid = (CURRENT_PID + 1) % PROCESS_LIST.len();

    if let Some(current) = &mut PROCESS_LIST[CURRENT_PID] {
        if !context.is_null() {
            current.context = context;
        }

        if let Some(next_process) = &mut PROCESS_LIST[next_pid] {
            CURRENT_PID = next_pid;

            //context_switch(next_proc.context, &*next_proc.context);
            return next_process.context;
        } else {
            return current.context;
        }
    } else {
        core::arch::asm!("hlt");
        loop {}
    }
}*/

extern "C" {
    fn context_switch(old_regs: *mut Context, new_regs: *const Context);
}

pub unsafe fn setup_processes() {
    let src = user_entry as *const u8;
    let dst = 0x620_000 as *mut u8;

    core::ptr::copy_nonoverlapping(src, dst, 4096);

    //
    //
    //

    let mut proc0 = create_process(0, 0x190_000);
    let proc1 = create_process(user_entry as u64, 0x7f0_000);
    let proc2 = create_process(keyboard_loop as u64, 0x700_000);

    proc0.status = Status::Idle;

    PROCESS_LIST = [Some(proc0), Some(proc1), Some(proc2)]
}

#[no_mangle]
extern "C" fn user_entry() -> ! {
    let mut i = 0;
    loop {
        unsafe {
            i += 1;
            rprintn!(i);
            rprint!("\n");

            core::arch::asm!("int 0x20");

            for _ in 0..50_000 {
                //core::arch::asm!("mov rdx, 0", "int 0x7f", "hlt");
                core::arch::asm!("pause");
            }
        }
    }
}

fn create_process(entry_point: u64, process_stack_top: u64) -> Process {
    Process {
        id: 0,
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
            cs: 0x08, // user code segment selector (RPL=3)
            //cs: 0x1B,      // user code segment selector (RPL=3)
            rflags: 0x202, // IF=1 (interrupt flag)
            rsp: process_stack_top,
            //ss: 0x23, // user stack segment selector
            ss: 0x10, // user stack segment selector
        },
        kernel_stack: [0; 4096],
    }
}
