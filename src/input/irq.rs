use core::sync::atomic::{AtomicUsize, Ordering};

use crate::task::scheduler;

const MAX_RECEPTORS: usize = 5;
const USER_KBUF_SIZE: usize = 256;

pub struct Subscriber {
    pub buf_ptr: u64,
    pub pid: usize,
    kbuf: [u8; USER_KBUF_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl Subscriber {
    pub const fn new() -> Self {
        Self {
            buf_ptr: 0,
            pid: 0,
            kbuf: [0; USER_KBUF_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Called from IRQ context.
    /// Writes `b` into the kernel ring buffer (for drain-syscall readers) and
    /// directly into `buf_ptr[0]` for programs that poll the mailbox slot.
    pub fn push_irq(&self, b: u8) {
        // Mailbox path: write to the user buffer only when the slot is empty.
        if self.buf_ptr != 0 {
            unsafe {
                if core::ptr::read_volatile(self.buf_ptr as *const u8) == 0 {
                    core::ptr::write_volatile(self.buf_ptr as *mut u8, b);
                }
            }
        }

        // Ring-buffer path: always enqueue into kbuf for drain-syscall readers.
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % USER_KBUF_SIZE;
        if next != self.tail.load(Ordering::Acquire) {
            unsafe {
                core::ptr::write_volatile(self.kbuf.as_ptr().add(head) as *mut u8, b);
            }
            self.head.store(next, Ordering::Release);
        }
    }

    /// Drain up to `len` bytes from the kernel ring buffer into `dst`.
    pub fn copy_to_user(&self, dst: *mut u8, len: usize) -> usize {
        let mut copied = 0usize;
        while copied < len {
            let tail = self.tail.load(Ordering::Relaxed);
            let head = self.head.load(Ordering::Acquire);
            if tail == head {
                break;
            }
            let byte = unsafe { core::ptr::read_volatile(self.kbuf.as_ptr().add(tail)) };
            unsafe { core::ptr::write_volatile(dst.add(copied), byte) };
            self.tail
                .store((tail + 1) % USER_KBUF_SIZE, Ordering::Release);
            copied += 1;
        }
        copied
    }

    pub fn available(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Relaxed);
        if head >= tail {
            head - tail
        } else {
            USER_KBUF_SIZE - tail + head
        }
    }

    pub fn clear(&self) {
        self.head.store(0, Ordering::Relaxed);
        self.tail.store(0, Ordering::Relaxed);
    }
}

pub static mut RECEPTORS: [Subscriber; MAX_RECEPTORS] = [
    Subscriber::new(),
    Subscriber::new(),
    Subscriber::new(),
    Subscriber::new(),
    Subscriber::new(),
];

pub fn pipe_subscribe(addr: u64) -> isize {
    let pid = unsafe { scheduler::get_current_pid() };

    unsafe {
        #[expect(static_mut_refs)]
        for s in RECEPTORS.iter_mut() {
            if s.pid == 0 {
                s.pid = pid;
                s.buf_ptr = addr;
                s.clear();

                rprint!("kbd: pid ");
                rprintn!(pid as u64);
                rprint!(" subscribed\n");
                return 0;
            }
        }
    }
    -1
}

pub fn pipe_unsubscribe(_addr: u64) -> isize {
    let pid = unsafe { scheduler::get_current_pid() };

    unsafe {
        #[expect(static_mut_refs)]
        for s in RECEPTORS.iter_mut() {
            if s.pid == pid {
                s.pid = 0;
                s.buf_ptr = 0;
                s.clear();
                return 0;
            }
        }
    }
    -1
}
