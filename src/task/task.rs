use x86_64::structures::idt::InterruptStackFrame;

#[derive(Clone, Copy)]
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
    rip: usize,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

pub struct Task {
    pub regs: Registers,
    //pub stack: &'static mut [u8],
    pub stack: u64,
    pub is_done: bool,
}

const MAX_TASKS: usize = 4;

#[unsafe(no_mangle)]
static mut TASKS: [Option<Task>; MAX_TASKS] = [None, None, None, None];
static mut CURRENT_TASK: usize = 0;

#[unsafe(no_mangle)]
static STACKS: [u64; 4] = [0x790000, 0x780000, 0x770000, 0x760000];

#[unsafe(no_mangle)]
extern "C" fn new_stack() -> u64 {
//extern "C" fn new_stack() -> &'static mut [u8] {
    //static mut STACKS: [[u8; 4096]; MAX_TASKS] = [[0; 4096]; MAX_TASKS];
    static mut NEXT: usize = 0;

    unsafe {
        let s = STACKS[NEXT];

        let ptr = s as *mut u8;
        for i in 0..4096 {
            ptr.add(i).write_volatile(0);
        }

        NEXT += 1;
        s
    }
}

fn add_task(entry: extern "C" fn()) {
    let stack = new_stack();
    let rsp = stack + 0x90000 - 8; // top of stack

    let regs = Registers {
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
        rip: entry as usize,
        cs: 0x08,
        rflags: 0x202,
        rsp,
        ss: 0x10,
    };

    unsafe {
        for slot in TASKS.iter_mut() {
            if slot.is_none() {
                *slot = Some(Task { regs, stack, is_done: false });
                break;
            }
        }
    }
}

extern "C" {
    fn context_switch(old: *mut Registers, new: *const Registers);
    //fn context_switch_kern(old: *mut Registers, new: *const Registers);
}

#[no_mangle]
pub fn status() {
    unsafe {
        print!("RUNNING TASKS\n");
        for i in 0..MAX_TASKS {
            if let Some(task) = &TASKS[i] {
                if task.is_done {
                    continue;
                }

                print!("Task ");
                printn!(i);
                print!("\n");
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn end_task(task_id: usize) {
    unsafe {
        if task_id != 0xff {
            if let Some(task) = &mut TASKS[task_id] {
                if task.is_done {
                    task.is_done = true;
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn schedule() {
    unsafe {
        // Get current task slot mutable reference
        let old_task_opt = &mut TASKS[CURRENT_TASK];

        // Start searching for the next runnable task
        let mut next = CURRENT_TASK;

        //for _ in 0..MAX_TASKS {
            next = (next + 1) % 2;

            // Check if next task exists and is not done
            if let Some(next_task) = &TASKS[next] {
                if !next_task.is_done {
                    // Check that old task exists as well
                    if let Some(old_task) = old_task_opt.as_mut() {
                        rprint!("[SCHEDULE] Switching from task ");
                        rprintn!(CURRENT_TASK);
                        rprint!(" to task ");
                        rprintn!(next);
                        rprint!("\n");

                        rprint!("Task ");
                        rprintn!(CURRENT_TASK);
                        rprint!(" registers:\nRIP: ");
                        rprintn!(old_task.regs.rip);
                        rprint!("\nRSP: ");
                        rprintn!(old_task.regs.rsp);
                        rprint!("\nSS: ");
                        rprintn!(old_task.regs.ss);
                        rprint!("\n\n");

                        rprint!("Task ");
                        rprintn!(next);
                        rprint!(" registers:\nRIP: ");
                        rprintn!(next_task.regs.rip);
                        rprint!("\nRSP: ");
                        rprintn!(next_task.regs.rsp);
                        rprint!("\nSS: ");
                        rprintn!(next_task.regs.ss);
                        rprint!("\n\n");

                        // Update current task index
                        CURRENT_TASK = next;

                        // Perform the context switch with valid pointers
                        context_switch(
                            &mut old_task.regs as *mut Registers,
                            &next_task.regs as *const Registers,
                        );

                        //break;
                    } else {
                        //println!("[SCHEDULE] Current task ");
                        //printn!(CURRENT_TASK); 
                        //println!(" is None")
                    }
                }
            } else {
                //print!("[SCHEDULE] Task slot ");
                //printn!(next);
                //println!(" is None")
                //}
    }
    }
}

#[unsafe(no_mangle)]
static mut PIPE: Option<super::pipe::Pipe> = None;

#[unsafe(no_mangle)]
extern "C" fn kern_task1() {
    let mut ch: u8 = 0;

    //println!("[TASK 1]: Start");

    loop {
        unsafe {
            if let Some(pipe) = PIPE.as_mut() {
                ch += 1;
                pipe.write(ch);

                if ch % 26 == 0 {
                    ch = 0;
                }

                for i in 0..10_000 {
                    core::arch::asm!("nop");
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn kern_task2() {
    //println!("[TASK 2]: Start");

    loop {
        unsafe {
            if let Some(pipe) = PIPE.as_mut() {
                let ch = pipe.read();

                if ch == 0x00 {
                    continue;
                }

                rprintb!( &[ch % 26 + 65] );
            }
        }
    }
}

#[no_mangle]
#[unsafe(link_section = ".user_task.task1")]
extern "C" fn user_task1() {
    #[unsafe(link_section = ".user_task.data1")]
    static msg1: [u8; 18]= *b"[TASK 1]: bonjour\n";

    loop {
        //print!("[TASK 1]: bonjour\n");

        unsafe {
            core::arch::asm!(
                "mov rdi, {0}",
                "mov rsi, {1:r}",
                "mov rax, 0x10",
                "int $0x7f",
                in(reg) msg1.as_ptr(),
                in(reg) msg1.len(),
            );
        }

        for _ in 0..50_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

#[no_mangle]
#[unsafe(link_section = ".user_task.task2")]
extern "C" fn user_task2() {
    #[unsafe(link_section = ".user_task.data2")]
    static msg2: [u8; 17] = *b"[TASK 2]: wowerz\n";

    loop {
        //print!("[TASK 1]: bonjour\n");

        unsafe {
            core::arch::asm!(
                "mov rdi, {0}",
                "mov rsi, {1:r}",
                "mov rax, 0x10",
                "int $0x7f",
                in(reg) msg2.as_ptr(),
                in(reg) msg2.len(),
            );
        }

        for _ in 0..50_000_000 {
            unsafe { core::arch::asm!("nop"); }
        }
    }
}

pub fn run_scheduler() {
    unsafe {
        PIPE = Some(super::pipe::Pipe::new(0));
    }

    add_task(kern_task1);
    add_task(kern_task2);
    //schedule();
}

