use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{input::port, task::scheduler};

const MAX_MOUSE_RECEPTORS: usize = 5;
const MOUSE_BUF_SIZE: usize = 64;

// PS/2 mouse packet reassembly state. IRQ12 fires once per byte;
// three bytes form one complete packet [flags, dx, dy].
static mut PHASE: usize = 0;
static mut PKT_BUF: [u8; 3] = [0; 3];

pub struct MouseSubscriber {
    pub pid: usize,
    pkts: [[u8; 3]; MOUSE_BUF_SIZE],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl MouseSubscriber {
    pub const fn new() -> Self {
        Self {
            pid: 0,
            pkts: [[0; 3]; MOUSE_BUF_SIZE],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    fn push(&self, raw: [u8; 3]) {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % MOUSE_BUF_SIZE;
        if next != self.tail.load(Ordering::Acquire) {
            unsafe {
                core::ptr::write_volatile(self.pkts.as_ptr().add(head) as *mut [u8; 3], raw);
            }
            self.head.store(next, Ordering::Release);
        }
    }

    pub fn drain_to(&self, dst: *mut u8, max_pkts: usize) -> usize {
        let mut copied = 0usize;
        while copied < max_pkts {
            let tail = self.tail.load(Ordering::Relaxed);
            let head = self.head.load(Ordering::Acquire);
            if tail == head {
                break;
            }
            let raw = unsafe { core::ptr::read_volatile(self.pkts.as_ptr().add(tail)) };
            unsafe {
                let out = dst.add(copied * 3);
                core::ptr::write_volatile(out, raw[0]);
                core::ptr::write_volatile(out.add(1), raw[1]);
                core::ptr::write_volatile(out.add(2), raw[2]);
            }
            self.tail
                .store((tail + 1) % MOUSE_BUF_SIZE, Ordering::Release);
            copied += 1;
        }
        copied
    }
}

pub static mut MOUSE_RECEPTORS: [MouseSubscriber; MAX_MOUSE_RECEPTORS] = [
    MouseSubscriber::new(),
    MouseSubscriber::new(),
    MouseSubscriber::new(),
    MouseSubscriber::new(),
    MouseSubscriber::new(),
];

/// Called from IRQ12 with each raw byte. Reassembles 3-byte PS/2 packets.
/// Bit 3 of byte 0 is always 1 — used to detect and recover sync loss.
pub fn push_byte(b: u8) {
    unsafe {
        if PHASE == 0 && b & 0x08 == 0 {
            return;
        }
        PKT_BUF[PHASE] = b;
        PHASE += 1;
        if PHASE == 3 {
            PHASE = 0;
            let raw = PKT_BUF;
            #[expect(static_mut_refs)]
            for s in MOUSE_RECEPTORS.iter() {
                if s.pid != 0 {
                    s.push(raw);
                }
            }
        }
    }
}

pub fn mouse_subscribe(_addr: u64) -> isize {
    let pid = unsafe { scheduler::get_current_pid() };
    unsafe {
        #[expect(static_mut_refs)]
        for s in MOUSE_RECEPTORS.iter_mut() {
            if s.pid == 0 {
                s.pid = pid;
                s.head.store(0, Ordering::Relaxed);
                s.tail.store(0, Ordering::Relaxed);
                return 0;
            }
        }
    }
    -1
}

pub fn mouse_unsubscribe() -> isize {
    let pid = unsafe { scheduler::get_current_pid() };
    unsafe {
        #[expect(static_mut_refs)]
        for s in MOUSE_RECEPTORS.iter_mut() {
            if s.pid == pid {
                s.pid = 0;
                return 0;
            }
        }
    }
    -1
}

pub fn mouse_drain(pid: usize, dst: *mut u8, max_bytes: usize) -> usize {
    let max_pkts = max_bytes / 3;
    unsafe {
        #[expect(static_mut_refs)]
        for s in MOUSE_RECEPTORS.iter() {
            if s.pid == pid {
                return s.drain_to(dst, max_pkts) * 3;
            }
        }
    }
    0
}

fn wait_write() {
    // Spin until the 8042 input buffer is empty (status bit 1 clear).
    while port::read_u8(0x64) & 0x02 != 0 {}
}

fn wait_read() {
    // Spin until the 8042 output buffer has data (status bit 0 set).
    while port::read_u8(0x64) & 0x01 == 0 {}
}

/// Initialise PS/2 mouse: enable auxiliary port on the 8042, unmask IRQ12
/// on the slave PIC, and instruct the mouse to start sending packets.
/// Must be called after the IDT is loaded so IRQ12 is handled.
pub fn init() {
    wait_write();
    port::write(0x64, 0xA8); // Enable Auxiliary Device

    // Read the 8042 Controller Configuration Byte (CCB) and set bit 1 to enable
    // mouse IRQ12. QEMU initializes the 8042 with bit 0 (keyboard IRQ1) set and
    // bit 1 (mouse IRQ12) CLEAR. Without explicitly setting bit 1, the 8042 never
    // asserts IRQ12 even if the mouse is enabled and sending data.
    wait_write();
    port::write(0x64, 0x20); // "Read CCB" command
    wait_read();
    let ccb = port::read_u8(0x60);
    rprint!("MOUSE INIT: 8042 CCB was ");
    rprintn!(ccb);
    rprint!("\n");
    wait_write();
    port::write(0x64, 0x60); // "Write CCB" command
    wait_write();
    port::write(0x60, ccb | 0x02); // bit 1 = enable mouse IRQ12

    // Unmask cascade IRQ2 on master PIC (bit 2 of port 0x21). Without this,
    // ALL slave PIC IRQs (8-15) are blocked — IRQ12 never fires, OBF stays
    // set after any mouse click and permanently blocks keyboard input.
    let master_mask = port::read_u8(0x21);
    rprint!("MOUSE INIT: master PIC mask was ");
    rprintn!(master_mask);
    rprint!("\n");
    port::write_u8(0x21, master_mask & !0x04);

    // Unmask IRQ12 on slave PIC (port 0xA1, bit 4).
    let slave_mask = port::read_u8(0xA1);
    rprint!("MOUSE INIT: slave PIC mask was ");
    rprintn!(slave_mask);
    rprint!("\n");
    port::write_u8(0xA1, slave_mask & !0x10);

    // Route 0xF4 (Enable Data Reporting) to the mouse via the 8042.
    wait_write();
    port::write(0x64, 0xD4);
    wait_write();
    port::write(0x60, 0xF4);

    // Discard ACK byte.
    wait_read();
    let _ = port::read_u8(0x60);
}
