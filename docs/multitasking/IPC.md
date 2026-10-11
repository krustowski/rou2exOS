# IPC

Inter-process communication techniques.

## Message Queue

Each process holds one `Port` (index 0). A `Port` contains a fixed-size circular
`Queue` of up to **64** `Message` entries. Networking and the legacy send syscall
share this queue.

```rust
struct Message {
    port_id:  usize,   // frame length when used for network delivery; otherwise 0
    src_pid:  usize,   // sender scheduler slot (despite the field name)
    dst_pid:  usize,   // target scheduler slot
    buf_addr: u64,     // payload address; network frames own a FRAME_BUF slot
}
```

### Legacy send path

1. The caller copies its payload into `MSG_BUF[0]` (512-byte kernel buffer).
2. The current syscall `0x36` constructs a `Message` with `buf_addr = 512`,
   rather than the address of `MSG_BUF[0]`.
3. `scheduler::push_msg(target_slot, msg)` attempts to enqueue it and wakes the
   target. A full queue or contended scheduler lock is not reported to the caller.

This is a legacy implementation limitation: the receive path treats `buf_addr`
as an address, so these messages do not deliver the copied payload correctly.
Argument 1 is also used as a scheduler slot, not a public PID. See
[syscall `0x36`](../abi/syscalls/port_networking.md#0x36-socket-send).
Network delivery uses owned frame buffers and a checked enqueue path instead.

### Receiving (syscall `0x35` / `receive_data`)

1. `scheduler::pop_msg(current_pid)` dequeues the front message.
2. If a message is available, the kernel validates and copies `port_id` bytes,
   or 512 bytes when `port_id` is zero, into the caller's buffer. Network frame
   storage is released after the copy. The return value is the copied length.
3. If the queue is empty and Argument 1 is zero, the call returns `0` immediately.
4. If the queue is empty and Argument 1 is nonzero, the caller is marked
   `Blocked` until delivery wakes it. The empty-queue call can return `0` after
   wakeup; call again to obtain the frame.

Each call consumes at most one message. There is no peek operation; both
blocking and non-blocking reads remove a queued message. The syscall receives
no destination capacity: reserve a 2 KiB buffer for Ethernet frames. See
[syscall `0x35`](../abi/syscalls/port_networking.md#0x35-socket-receive).

## Network Delivery (NIC → userland)

The selected RTL8139 or Intel NIC is polled in `scheduler_schedule` before
the round-robin pick. `netdrv::poll_and_deliver` handles up to 16 frames per
tick, routes each to a TCP port owner, tunnel daemon or global driver, and
queues it with `try_push_msg`. Each frame owns one of 64 separate 2 KiB
buffers until received or its owner exits. A frame that cannot be queued is
held at the ring head and retried, with a 200-tick limit for an unresponsive
receiver. See [Receive Path](../networking/overview.md#receive-path-ethernet).

[![Network delivery owns one frame buffer until the process receives it; enqueue succeeds before the NIC consumes the frame, and unavailable queues hold the ring head for retry.](../assets/diagrams/network-receive.svg)](../assets/diagrams/network-receive.svg)

*Network message ownership. The frame buffer lives until receive or process
termination; a message queue entry carries its address and length.*

## Keyboard Pipe (IRQ 1 → userland)

User processes can subscribe to raw PS/2 scancodes via syscall `0x03` (pipe subscribe). The IRQ 1 handler writes each scancode byte into all subscribed circular buffers. A process reads its buffer with syscall `0x03 / 0x03` (pipe read). This is a polled, non-blocking path — the process must periodically call `pipe_read` rather than blocking.

---

Related documentation: [Kernel Internals](../kernel.md), [Scheduler](scheduler.md), [Networking Overview](../networking/overview.md), [IPC Syscalls](../abi/syscalls/port_networking.md).
