use crate::task::{
    context::Context,
    queue::{Message, Queue},
};

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
    Blocked,
    Crashed,
    Idle,
    Dead,
}

#[derive(Debug)]
#[repr(C)]
pub struct Port {
    pub id: usize,
    pub block_msg: Option<Message>,
    pub bind: bool,
    pub queue: Queue,
}

#[derive(Debug)]
#[repr(C)]
pub struct Process {
    pub id: usize,
    pub name: [u8; 16],
    pub mode: Mode,
    pub status: Status,
    pub last_rsp: u64,
    pub kernel_stack: &'static [u8; STACK_SIZE],
    pub ports: [Port; 1],
    pub stack_top: u64,
    //pub context: Context,
}

const STACK_SIZE: usize = 32768;
static mut KSTACK_POOL: [[u8; STACK_SIZE]; 5] = [
    [0; STACK_SIZE],
    [0; STACK_SIZE],
    [0; STACK_SIZE],
    [0; STACK_SIZE],
    [0; STACK_SIZE],
];

impl Process {
    pub fn new(
        id: usize,
        name_slice: [u8; 16],
        mode: Mode,
        entry_point: u64,
        process_stack_top: u64,
    ) -> Process {
        let mut name: [u8; 16] = [0; 16];

        if let Some(slice) = name.get_mut(..name_slice.len()) {
            (*slice)[..name_slice.len()].copy_from_slice(&name_slice);
        }

        Process {
            id,
            name,
            mode,
            status: Status::Ready,
            last_rsp: 0,
            //context: Context::new(entry_point, code_segment, process_stack_top, stack_segment),
            stack_top: process_stack_top,
            kernel_stack: unsafe { &KSTACK_POOL[id % 5] },
            ports: [Port {
                id: 0,
                block_msg: None,
                bind: false,
                queue: Queue::new(),
            }],
        }
    }

    pub fn get_pid(&self) -> usize {
        self.id
    }

    pub fn get_name(&self) -> [u8; 16] {
        self.name
    }
}
