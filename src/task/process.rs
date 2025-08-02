#[repr(C)]
pub struct Registers {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,

    // iretq stack frame
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

pub struct Process {
    regs: Registers,
    kernel_stack: [u8; 4096],
    //pid: u16,
    //page_tables: ...
}

static mut PROCESSES: [Option<Process>; 2] = [None, None];
static mut CURRENT_PID: usize = 0;

pub fn schedule() {
    unsafe {
        let next_pid = (CURRENT_PID + 1) % PROCESSES.len();

        if let Some(next_proc) = &mut PROCESSES[next_pid] {
            context_switch(&mut PROCESSES[CURRENT_PID].as_mut().unwrap().regs, &next_proc.regs);
            CURRENT_PID = next_pid;
        }
    }
}

extern "C" {
    fn context_switch(old_regs: *mut Registers, new_regs: *const Registers);
}

fn create_process(entry_point: u64, process_stack_top: u64) -> Process {
    let mut p = Process {
        regs: Registers {
            r15: 0, r14: 0, r13: 0, r12: 0,
            r11: 0, r10: 0, r9: 0, r8: 0,
            rdi: 0, rsi: 0, rbp: 0, rdx: 0,
            rcx: 0, rbx: 0, rax: 0,
            rip: entry_point,
            cs: 0x1B,          // user code segment selector (RPL=3)
            rflags: 0x202,     // IF=1 (interrupt flag)
            rsp: process_stack_top,
            ss: 0x23,          // user stack segment selector
        },
        kernel_stack: [0; 4096],
    };
    p
}

