use spin::Mutex;

use super::{
    process::{Mode, Process, Status, MAX_PROCESSES},
    queue::Message,
};

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

#[repr(C)]
pub struct Scheduler {
    processes: [Option<Process>; MAX_PROCESSES],
    current_pid: usize,
    next_free_pid: usize,
}

extern "C" {
    static mut tss64: crate::init::idt::Tss64;
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            processes: [None, None, None, None, None],
            current_pid: 0,
            next_free_pid: 0,
        }
    }

    pub unsafe fn schedule(&mut self, old: *mut u64) -> *mut u64 {
        let mut next = self.current_pid;
        let start = next;

        loop {
            next = (next + 1) % self.processes.len();

            if next == start {
                return old;
            }

            let next_proc = self.processes[next].as_ref();

            if next_proc.is_none() {
                continue;
            }

            if let Some(proc) = next_proc {
                if proc.status == Status::Dead {
                    self.processes[next].take();
                    continue;
                }

                if matches!(
                    proc.status,
                    Status::Blocked | Status::Crashed | Status::Idle
                ) {
                    continue;
                }

                break;
            }
        }

        let curr_proc = self.processes[self.current_pid].as_mut().unwrap();

        // Save the current process' context
        //self.processes[self.current_pid].as_mut().unwrap().context = *old;
        //curr_proc.context = *old;
        //copy_nonoverlapping(old, &mut curr_proc.context, 1);
        curr_proc.last_rsp = old as u64;

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

        let next_proc = self.processes[next].as_mut().unwrap();

        next_proc.status = Status::Running;
        self.current_pid = next;

        // Prepare a custom kernel stack for the next process
        let kern_stack = &next_proc.kernel_stack;

        // Update the RSP0 field in TSS; align it to 16bits
        tss64.rsp0 = (kern_stack.as_ptr() as u64) + kern_stack.len() as u64;
        tss64.rsp0 &= !0x0F;

        rprint!("RETURNING RSP: ");
        rprintn!(next_proc.last_rsp);
        rprint!(", RIP: ");
        rprintn!(*((next_proc.last_rsp + 120) as *const u64)); // should be RIP
        rprint!(", CS: ");
        rprintn!(*((next_proc.last_rsp + 128) as *const u64)); // should be CS
        rprint!(", NEXT PID: ");
        rprintn!(next);
        rprint!(", RFLAGS: ");
        rprintn!(*((next_proc.last_rsp + 136) as *const u64)); // should be RFLAGS
        rprint!("\n");

        //&mut next_proc.context
        next_proc.last_rsp as *mut u64
    }

    fn check_pid(&self, pid: usize) -> bool {
        if pid >= self.processes.len() {
            return false;
        }

        if self.processes[pid].is_some() {
            return true;
        }

        false
    }

    pub fn push_msg(&mut self, pid: usize, msg: Message) {
        if !self.check_pid(pid) {
            return;
        }

        self.processes[pid].as_mut().unwrap().ports[0]
            .queue
            .push(msg);

        // Wake the target process to fetch new message
        self.set_status(pid, Status::Ready);
    }

    pub unsafe fn pop_msg(&mut self, pid: usize) -> Option<Message> {
        if !self.check_pid(pid) {
            return None;
        }

        self.processes[pid].as_mut().unwrap().ports[0].queue.pop()
    }

    pub fn set_status(&mut self, mut pid: usize, status: Status) {
        if pid == 0xff {
            pid = self.current_pid;
        }

        if !self.check_pid(pid) {
            return;
        }

        self.processes[pid].as_mut().unwrap().status = status;
    }

    pub fn kill(&mut self, pid: usize) {
        self.set_status(pid, Status::Dead);

        rprint!("KILL PID ");
        rprintn!(pid);
        rprint!("\n\n");
    }

    pub fn block(&mut self, pid: usize, msg: Message) {
        if !self.check_pid(pid) {
            return;
        }

        self.set_status(pid, Status::Blocked);
        self.processes[pid].as_mut().unwrap().ports[0].block_msg = Some(msg);
    }

    pub unsafe fn list_processes(&self) {
        print!("RUNNING PROCESSES\n");

        for process in self.processes.iter() {
            if process.is_none() {
                continue;
            }

            if let Some(proc) = process {
                printn!(proc.id);
                print!("   ");
                printb!(&proc.name);
                print!("   ");

                match proc.status {
                    Status::Ready => {
                        print!(" (Ready)");
                    }
                    Status::Running => {
                        print!(" (Running)");
                    }
                    Status::Idle => {
                        print!(" (Idle)");
                    }
                    Status::Blocked => {
                        print!(" (Blocked)");
                    }
                    Status::Crashed => {
                        print!(" (Crashed)");
                    }
                    Status::Dead => {
                        print!(" (Dead)");
                    }
                }

                print!("\n");
            }
        }
    }

    fn get_next_pid(&mut self) -> usize {
        self.next_free_pid += 1;
        self.next_free_pid - 1
    }

    pub fn get_current_pid(&self) -> usize {
        self.current_pid
    }

    pub unsafe fn new_process(
        &mut self,
        name: [u8; 16],
        mode: Mode,
        entry: u64,
        stack_top: u64,
    ) -> usize {
        let pid: usize = self.get_next_pid();

        let mut proc: Option<Process> = None;
        let mut pos: usize = 0;
        let mut found_slot = false;

        for slot in self.processes.iter() {
            if slot.is_some() {
                pos += 1;
                continue;
            }
            found_slot = true;
            break;
        }

        if !found_slot || pos >= MAX_PROCESSES {
            return 0xff;
        }

        {
            // Process::new takes `slot` (not pid) so the kernel stack pool is
            // indexed by slot position — each slot always owns the same stack
            // entry regardless of how many times it has been recycled.
            proc = Some(Process::new(pid, pos, name, mode, entry, stack_top));

            unsafe {
                let kstack_top = proc.as_mut().unwrap().kernel_stack.as_ptr().add(32768) as u64;
                let mut sp = kstack_top;

                sp &= !0xF;

                let code_segment: u64;
                let stack_segment: u64;

                match mode {
                    Mode::User => {
                        code_segment = 0x1b;
                        stack_segment = 0x23;
                    }
                    _ => {
                        code_segment = 0x08;
                        stack_segment = 0x10;
                    }
                }

                // SS
                sp -= 8;
                *(sp as *mut u64) = stack_segment;

                // RSP
                sp -= 8;
                *(sp as *mut u64) = match mode {
                    Mode::User => stack_top,
                    _ => kstack_top,
                };

                // RFLAGS
                sp -= 8;
                *(sp as *mut u64) = 0x202;

                // CS
                sp -= 8;
                *(sp as *mut u64) = code_segment;

                // RIP
                sp -= 8;
                *(sp as *mut u64) = entry;

                // push registers (zero them)
                for _ in 0..15 {
                    sp -= 8;
                    *(sp as *mut u64) = 0;
                }

                proc.as_mut().unwrap().last_rsp = sp;
            }
        }

        rprint!("NEW PROCESS: ");
        rprintb!(&proc.as_ref().unwrap().name);
        rprint!(", INIT RSP: ");
        rprintn!(proc.as_ref().unwrap().last_rsp);
        rprint!(", ");
        rprint!("STACK BASE: ");
        rprintn!(proc.as_ref().unwrap().kernel_stack.as_ptr() as u64);
        rprint!(", pid: ");
        rprintn!(proc.as_ref().unwrap().id);
        rprint!("\n");

        self.processes[pos] = proc;

        pid
    }
}

#[no_mangle]
pub unsafe extern "C" fn scheduler_schedule(mut old: *mut u64) -> *mut u64 {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        old = sch.schedule(old);
    }

    crate::input::port::write(0x20, 0x20);
    old
}

pub unsafe fn kill(pid: usize) {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        sch.kill(pid);
    }
}

pub unsafe fn block(pid: usize, msg: Message) {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        sch.block(pid, msg);
    }
}

pub unsafe fn wake(pid: usize) {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        sch.set_status(pid, Status::Ready);
    }
}

pub unsafe fn idle(pid: usize) {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        sch.set_status(pid, Status::Idle);
    }
}

pub unsafe fn crash(pid: usize) {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        sch.set_status(pid, Status::Crashed);
    }
}

pub unsafe fn list_processes() {
    if let Some(sch) = SCHEDULER.try_lock() {
        sch.list_processes();
    }
}

pub unsafe fn new_process(name: [u8; 16], mode: Mode, entry: u64, stack_top: u64) -> usize {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        return sch.new_process(name, mode, entry, stack_top);
    }

    0xff
}

pub unsafe fn get_current_pid() -> usize {
    if let Some(sch) = SCHEDULER.try_lock() {
        return sch.get_current_pid();
    }

    0xff
}

pub unsafe fn push_msg(pid: usize, msg: Message) {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        sch.push_msg(pid, msg)
    }
}

pub unsafe fn pop_msg(pid: usize) -> Option<Message> {
    if let Some(mut sch) = SCHEDULER.try_lock() {
        return sch.pop_msg(pid);
    }

    None
}
