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
    //pub rsp: u64,
    //pub ss: u64,
}

impl Context {
    pub fn new(rip: u64, cs: u64, _rsp: u64, _ss: u64) -> Context {
        Context {
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
            rip,
            cs,
            rflags: 0x202, // IF = 1
                           //rsp,
                           //ss,
        }
    }
}
