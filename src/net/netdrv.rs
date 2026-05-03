/*
 *  netdrv.rs — kernel-side glue between the RTL8139 NIC and the userland ETH driver process.
 *
 *  The ETH userland process registers itself via syscall 0x37 (arg1 = 0) as the global driver:
 *  it receives all frames that are not claimed by a port-specific service.  Application services
 *  (e.g. garn, tnt) register with syscall 0x37 (arg1 = TCP port) to receive only frames whose
 *  TCP destination port matches the registered value.
 *
 *  On each timer tick the scheduler calls poll_and_deliver(), which reads one frame from the NIC,
 *  routes it to the right process, and pushes an IPC message that wakes the blocked receive_data().
 */

use crate::init::config::SYSTEM_CONFIG;
use crate::net::rtl8139;
use crate::task::queue::Message;
use crate::task::scheduler;

const NO_PID: usize = 0xff;
const MAX_PORT_BINDINGS: usize = 16;

static mut NET_FRAME_BUF: [u8; 2048] = [0u8; 2048];

/// PID of the global Ethernet driver (handles ARP, ICMP, and unbound TCP ports).
static mut NET_DRV_PID: usize = NO_PID;

/// Per-port registry: (tcp_dest_port, pid).  A pid of NO_PID marks a free slot.
static mut PORT_REGISTRY: [(u16, usize); MAX_PORT_BINDINGS] = [(0, NO_PID); MAX_PORT_BINDINGS];

/// Called by syscall 0x37 with arg1 = 0. Stores the calling PID as the global Ethernet driver
/// process and initialises the RTL8139 hardware.  Idempotent: if a driver is already registered
/// the call is a no-op so that port-specific services (GARN, TNT) can safely call net_register()
/// before net_bind_port() without displacing an already-running global driver.
pub unsafe fn register_driver(pid: usize) {
    if NET_DRV_PID != NO_PID {
        return;
    }
    NET_DRV_PID = pid;
    rtl8139::rtl8139_init();

    // Cache MAC in SYSTEM_CONFIG so ScNetStatus can read it without re-probing PCI.
    let mac = rtl8139::read_mac_addr();
    if let Some(mut sc) = SYSTEM_CONFIG.try_lock() {
        sc.set_mac(mac);
    }

    rprint!("netdrv: registered pid=");
    rprintn!(pid as u64);
    rprint!(" io_base=0x");
    rprintn!(rtl8139::RTL8139_IO_BASE as u64);
    rprint!("\n");
}

/// Called by syscall 0x37 with arg1 = TCP port. Registers the calling PID to receive frames
/// whose TCP destination port equals `port`. Updates an existing entry for the same port if
/// one exists, otherwise takes the first free slot.
pub unsafe fn bind_port(port: u16, pid: usize) {
    for i in 0..MAX_PORT_BINDINGS {
        if PORT_REGISTRY[i].0 == port || PORT_REGISTRY[i].1 == NO_PID {
            PORT_REGISTRY[i] = (port, pid);

            rprint!("netdrv: port ");
            rprintn!(port as u64);
            rprint!(" bound to pid=");
            rprintn!(pid as u64);
            rprint!("\n");
            return;
        }
    }
    /* Table full — overwrite slot 0 */
    PORT_REGISTRY[0] = (port, pid);
}

pub unsafe fn get_driver_pid() -> usize {
    NET_DRV_PID
}

/// Fill `ports[0..n]` with active TCP port bindings; set `*n_ports` to the count.
pub unsafe fn fill_port_bindings(n_ports: &mut u8, ports: &mut [u16; 16]) {
    let mut count = 0u8;
    for i in 0..MAX_PORT_BINDINGS {
        if PORT_REGISTRY[i].1 != NO_PID {
            if (count as usize) < 16 {
                ports[count as usize] = PORT_REGISTRY[i].0;
                count += 1;
            }
        }
    }
    *n_ports = count;
}

/// Copy a frame from NET_FRAME_BUF to a userland pointer.
pub unsafe fn copy_frame_out(dst: *mut u8, len: usize) {
    core::ptr::copy_nonoverlapping(NET_FRAME_BUF.as_ptr(), dst, len);
}

/// Extract the TCP destination port from a raw Ethernet frame.
/// Returns None if the frame is not IPv4/TCP or is too short.
fn tcp_dest_port(frame: &[u8]) -> Option<u16> {
    if frame.len() < 14 {
        return None;
    }
    let ethertype = u16::from_be_bytes([frame[12], frame[13]]);
    if ethertype != 0x0800 {
        return None; /* not IPv4 */
    }
    if frame.len() < 14 + 20 {
        return None;
    }
    let protocol = frame[14 + 9];
    if protocol != 6 {
        return None; /* not TCP */
    }
    let ihl = (frame[14] & 0x0f) as usize * 4;
    let tcp_start = 14 + ihl;
    if frame.len() < tcp_start + 4 {
        return None;
    }
    Some(u16::from_be_bytes([frame[tcp_start + 2], frame[tcp_start + 3]]))
}

/// Look up the PID registered for a TCP destination port.
fn lookup_port(port: u16) -> Option<usize> {
    unsafe {
        for i in 0..MAX_PORT_BINDINGS {
            if PORT_REGISTRY[i].1 != NO_PID && PORT_REGISTRY[i].0 == port {
                return Some(PORT_REGISTRY[i].1);
            }
        }
    }
    None
}

/// Called from scheduler_schedule() on every timer tick.
/// Polls the NIC; if a frame is ready, routes it to the port-specific service that registered
/// for its TCP destination port, falling back to the global driver for everything else.
pub unsafe fn poll_and_deliver() {
    if NET_DRV_PID == NO_PID {
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

        /* Route TCP frames to the registered port-specific service; all other traffic
         * (ARP, ICMP, unregistered ports) goes to the global Ethernet driver. */
        let dest_pid = tcp_dest_port(&tmp[..len])
            .and_then(|port| lookup_port(port))
            .unwrap_or(NET_DRV_PID);

        let msg = Message::new(len, 0, dest_pid, NET_FRAME_BUF.as_ptr() as u64);
        scheduler::push_msg(dest_pid, msg);
    }
}
