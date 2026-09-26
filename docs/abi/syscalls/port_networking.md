# Port I/O and Networking

## 0x30 (Send value to port)

Write one byte to a hardware I/O port. Both args are pointers — the kernel dereferences them.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to port number (`*const u16`) | pointer to value (`*const u32`; low byte written) | ✅ |

## 0x31 (Receive value from port)

Read a 32-bit value from a hardware I/O port. Both args are pointers — the kernel dereferences them.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to port number (`*const u16`) | pointer to output (`*mut u32`; receives the result) | ✅ |

## 0x32 (Serial port)

UART port COM1.

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|-----|
| `0x01` | `0x00` | Serial port initialization. | ✅ |
| `0x02` | pointer to value | Read from the serial port. | ✅ |
| `0x03` | pointer to value | Write to the serial port. | ✅ |

## 0x33 (Create packet)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|-----|
| `0x01` | pointer to buffer | Create an IPv4 packet. | ✅ |
| `0x02` | pointer to buffer | Create an ICMP packet. | ✅ |
| `0x03` | pointer to buffer | Create a TCP packet. | ✅ |

## 0x34 (Send frame/packet)

| Argument 1 | Argument 2 | Meaning | Implemented |
|------------|------------|-------------|---------|
| `0x01` | pointer to buffer | Send an IPv4 packet (derives frame length from the IP header). | ✅ |
| `0x04` | pointer to raw Ethernet frame | Send a raw Ethernet frame. Length is derived from the EtherType field (`0x0800` = IPv4, `0x0806` = ARP). | ✅ |

## 0x35 (Socket receive)

Pops a message from the calling process' MQ. Non-blocking returns `0` immediately if the queue is empty; blocking suspends the process until a frame arrives.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x00` = non-blocking, non-zero = blocking | pointer to buffer | ✅  |

## 0x36 (Socket send)

Copies 512 bytes from the buffer and pushes a message to the target process' MQ, then wakes the target. 

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| target process PID | pointer to buffer | ✅  |

## 0x37 (Register Ethernet driver, bind TCP port)

Ethernet driver registration, or port binding. Both are released when the calling process exits, is killed or crashes.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| TCP port number (0 for global driver) | unused | ✅ |

## 0x38 (Get networking status)

Query network status (read-only). Writes `{ mac[6], ip[4], drv_active, n_ports, ports[16] }` into the struct. 

Returns `InvalidInput` on invalid pointer, `Ok` otherwise.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| pointer to `NetStatus` struct | *unused* | ✅ |

## 0x3d (Network configuration)

The machine's network configuration: IPv4 address, netmask, gateway, the gateway's MAC, DNS and the card's MAC, as the global Ethernet driver (`eth`) got them by DHCP or was given them. Every network stack reads it rather than assuming addresses of its own. The gateway's MAC matters most: a process that is not the global driver cannot ARP for it, because the replies go to the driver.

With Argument 1 `0x01` the kernel fills the [`NetConfig`](../type_definitions.md#netconfig-syscall-0x3d) at Argument 2; the IP and MAC come from the system configuration (the same ones `0x01` and `0x38` report), the rest is what the driver last set. Unset fields are zero.

With Argument 1 `0x02` the calling process sets it from the struct at Argument 2. Only the registered global driver (`0x37` with port 0) may; anyone else gets `InvalidInput`. The IP is stored as `0x01`/`0x02` would store it; the MAC is the card's and is ignored. `Busy` means the system configuration was locked that instant: try again.

Returns `InvalidInput` on an invalid pointer or Argument 1, `Ok` otherwise.

| Argument 1 | Argument 2 | Implemented |
|------------|------------|-------------|
| `0x01` read, `0x02` set | pointer to `NetConfig` struct | ✅ |
