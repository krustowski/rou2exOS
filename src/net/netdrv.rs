/*
 *  netdrv.rs — kernel-side glue between the RTL8139 NIC and the userland ETH driver process.
 *
 *  The ETH userland process registers itself via syscall 0x37. On each timer tick the
 *  scheduler calls poll_and_deliver(), which reads one frame from the NIC and pushes it to
 *  the ETH task's IPC queue, waking the blocked receive_data() call.
 */

use crate::net::rtl8139;
use crate::task::queue::Message;
use crate::task::scheduler;

static mut NET_FRAME_BUF: [u8; 2048] = [0u8; 2048];
static mut NET_DRV_PID: usize = 0xff;

/// Called by syscall 0x37. Stores the calling PID as the Ethernet driver process and
/// initialises the RTL8139 hardware.
pub unsafe fn register_driver(pid: usize) {
    NET_DRV_PID = pid;
    rtl8139::rtl8139_init();

    rprint!("netdrv: registered pid=");
    rprintn!(pid as u64);
    rprint!(" io_base=0x");
    rprintn!(rtl8139::RTL8139_IO_BASE as u64);
    rprint!("\n");
}

pub unsafe fn get_driver_pid() -> usize {
    NET_DRV_PID
}

/// Copy a frame from NET_FRAME_BUF to a userland pointer.
pub unsafe fn copy_frame_out(dst: *mut u8, len: usize) {
    core::ptr::copy_nonoverlapping(NET_FRAME_BUF.as_ptr(), dst, len);
}

/// Called from scheduler_schedule() on every timer tick.
/// Polls the NIC; if a frame is ready, copies it into NET_FRAME_BUF and pushes
/// an IPC message to the registered ETH driver process.
pub unsafe fn poll_and_deliver() {
    if NET_DRV_PID == 0xff {
        return;
    }

    let mut tmp: [u8; 2048] = [0u8; 2048];

    if let Some(len) = rtl8139::receive_frame(&mut tmp) {
        if len == 0 || len > 2048 {
            return;
        }

        rprint!("netdrv: frame len=");
        rprintn!(len as u64);
        rprint!("\n");

        core::ptr::copy_nonoverlapping(tmp.as_ptr(), NET_FRAME_BUF.as_mut_ptr(), len);

        // port_id carries the frame length; buf_addr points to NET_FRAME_BUF
        let msg = Message::new(len, 0, NET_DRV_PID, NET_FRAME_BUF.as_ptr() as u64);
        scheduler::push_msg(NET_DRV_PID, msg);
    }
}
