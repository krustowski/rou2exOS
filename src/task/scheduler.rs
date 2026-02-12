use spin::Mutex;

use super::process::{Context, Process, Status};

const MAX_PROCESSES: usize = 5;

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub struct Scheduler {
    processes: [Option<Process>; MAX_PROCESSES],
    current_pid: usize,
    next_free_pid: usize,
}

#[no_mangle]
pub unsafe extern "C" fn scheduler_schedule(old: *mut Context) -> *mut Context {
    let mut sch = SCHEDULER.lock();
    sch.schedule(old) as *mut Context
}

extern "C" {
    static mut tss64: crate::init::idt::Tss64;
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            processes: [None; MAX_PROCESSES],
            current_pid: 0,
            next_free_pid: 0,
        }
    }

    pub unsafe fn schedule(&mut self, old: *mut Context) -> *mut Context {
        //old;

        let mut next = (self.current_pid + 1) % self.processes.len();

        loop {
            if let Some(next_proc) = self.processes[next] {
                if next_proc.status == Status::Dead {
                    self.processes[next] = None;
                }

                if !matches!(
                    next_proc.status,
                    Status::Blocked | Status::Crashed | Status::Idle
                ) {
                    break;
                }

                next = (next + 1) % self.processes.len();
            }
        }

        let mut curr_proc = self.processes[self.current_pid].unwrap();

        // Save the current process' context
        curr_proc.context = *old;

        if !matches!(
            curr_proc.status,
            Status::Blocked | Status::Crashed | Status::Dead | Status::Idle
        ) {
            // Mark the current process as runnable, so it can be picked by the scheduler again
            curr_proc.status = Status::Ready;
        }

        // Do not wait for the next cycle to clean a dead process
        if curr_proc.status == Status::Dead {
            self.processes[self.current_pid] = None;
        }

        let mut next_proc = self.processes[next].as_mut().unwrap();

        next_proc.status = Status::Running;
        self.current_pid = next;

        // Prepare a custom kernel stack for the next process
        let kern_stack = next_proc.kernel_stack;

        // Update the RSP0 field in TSS; align it to 16bits
        tss64.rsp0 = (kern_stack.as_ptr() as u64) + kern_stack.len() as u64;
        tss64.rsp0 &= !0x0F;

        crate::input::port::write(0x20, 0x20);
        &mut next_proc.context
    }
}
