# IPC

## Message Queue

Each process holds one `Port` (index 0). A `Port` contains a fixed-size circular `Queue` of up to 10 `Message` entries.

```rust
struct Message {
    port_id:  usize,   // frame length when used for network delivery; otherwise 0
    src_pid:  usize,   // sender PID
    dst_pid:  usize,   // target PID
    buf_addr: u64,     // address of the payload buffer (in kernel .bss MSG_BUF)
}
```

### Sending (syscall `0x36` / `send_data`)

1. The caller copies its payload into `MSG_BUF[0]` (512-byte kernel buffer).
2. A `Message` is constructed with `buf_addr = MSG_BUF[0].as_ptr()`.
3. `scheduler::push_msg(target_pid, msg)` enqueues the message and transitions the target to `Ready` if it was `Blocked`.

### Receiving (syscall `0x35` / `receive_data`)

1. `scheduler::pop_msg(current_pid)` dequeues the front message.
2. If a message is available, the payload is `copy_nonoverlapping` into the caller's user-space buffer and the frame length is returned.
3. If the queue is empty, `scheduler::block(pid, sentinel_msg)` transitions the caller to `Blocked`. The scheduler will not run it again until a message arrives and `wake` is called by the sender.

This is a blocking, single-message-at-a-time rendezvous. There is no non-blocking peek; the caller blocks until data arrives.

## Network Delivery (RTL8139 → userland driver)

The NIC driver polls for incoming Ethernet frames in `scheduler_schedule` (before the round-robin pick, once per PIT tick via `netdrv::poll_and_deliver`). When a frame arrives it is placed in the registered driver process's message queue using the same `push_msg` mechanism, waking the driver if it was blocked on `receive_data`.

## Keyboard Pipe (IRQ 1 → userland)

User processes can subscribe to raw PS/2 scancodes via syscall `0x03` (pipe subscribe). The IRQ 1 handler writes each scancode byte into all subscribed circular buffers. A process reads its buffer with syscall `0x03 / 0x03` (pipe read). This is a polled, non-blocking path — the process must periodically call `pipe_read` rather than blocking.
