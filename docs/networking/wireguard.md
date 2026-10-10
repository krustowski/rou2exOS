# WireGuard (`wgd`)

`wgd` is a standalone C WireGuard daemon for r2. It runs alongside `eth`, supports one peer and IPv4, and reads its configuration once at startup. It can answer encrypted pings to r2's tunnel address, carry connections to r2 TCP services, and send shell probes to hosts in the peer's `AllowedIPs` subnets.

The daemon implements the protocol and cryptography in userland using a pinned BSD-licensed `wireguard-lwip` core, without a lwIP dependency. The kernel supplies packet routing and authenticated-frame injection through [syscall `0x44`](../abi/syscalls/port_networking.md#0x44-userspace-ipv4-tunnel). See the [daemon source and development guide](https://github.com/krustowski/rou2exOS-apps/tree/master/c/wgd) for build and interoperability tests.

## Build and prerequisites

Build and boot the updated kernel and applications together; copying `wgd.elf` onto an old kernel is insufficient. The daemon requires tunnel ABI 2, whose capability value is `0x52325748`. Both text and graphics kernels support it.

The kernel's image build rebuilds `wgd`, `sh` and `tnt`, and relinks `chat` with libcr2's TCP reply-address fix. It includes `/bin/wgd.elf` and `/opt/wgd/wgd.cfg.example` in the ISO and boot archive. Relink other existing libcr2 TCP servers before accessing them through the tunnel. See [Build and Run](../build.md) for image construction.

Before starting the daemon, `eth` must publish a physical IPv4 address and gateway. The tunnel address must differ from that physical address. Set the RTC to valid UTC time for WireGuard handshake timestamps. A CPU without RDSEED or RDRAND can use the [entropy fallback](#entropy-on-older-cpus).

## Configuration

Generate a separate private/public key pair for r2 and for its peer using the host's WireGuard tools:

```sh
umask 077
wg genkey > r2.key
wg pubkey < r2.key > r2.pub
wg genkey > peer.key
wg pubkey < peer.key > peer.pub
```

Copy the bundled example to a writable volume and replace its placeholders. For example, to probe `10.4.6.68` through a remote router:

```ini
[Interface]
PrivateKey = <contents of r2.key>
Address = 10.77.0.1/32
ListenPort = 51820

[Peer]
PublicKey = <contents of peer.pub>
Endpoint = 192.0.2.10:51820
AllowedIPs = 10.77.0.2/32, 10.4.6.0/24
PersistentKeepalive = 25
```

`192.0.2.10` is a documentation address; replace it with the remote peer's reachable physical IPv4 address. The example keys are invalid placeholders. Keep the completed private configuration out of version control.

On the remote peer, configure its private key, r2's public key, and `AllowedIPs = 10.77.0.1/32` for r2. For subnet access, that peer must forward traffic towards `10.4.6.0/24`, and replies must have a route back to `10.77.0.1`. Permit WireGuard UDP and ICMP in the relevant firewalls. Set the peer's tunnel MTU to 1420.

| Setting | Accepted value |
|---------|----------------|
| `[Interface] PrivateKey` | Required base64 WireGuard private key |
| `Address` | One IPv4 host address, optionally `/0`–`/32`; defaults to `/32`. Its prefix does not install routes. |
| `ListenPort` | UDP port; defaults to 51820 |
| `[Peer] PublicKey` | Required base64 peer public key |
| `AllowedIPs` | Required; up to 16 comma-separated IPv4 addresses or CIDR prefixes, including `/0`. Bare addresses mean `/32`; networks are normalized. |
| `PresharedKey` | Optional base64 key, identical on both peers |
| `Endpoint` | Optional numeric IPv4 address and UDP port, `address:port`; enables r2 to initiate the tunnel |
| `PersistentKeepalive` | 0–60 seconds; defaults to 0 (disabled) |

There must be exactly one peer. Files are limited to 2048 bytes; unknown options, duplicate fields, invalid keys or addresses, and embedded NUL bytes are rejected. Editing the file requires restarting the daemon.

Pass the configuration path explicitly. Without one, `wgd` tries `WGD.CFG` in its working directory, `/mnt/fat/WGD.CFG`, then `/mnt/tar/opt/wgd/wgd.cfg`. The bundled `.example` is not a default configuration. `/mnt/tmp` is a RAM disk and its contents disappear at reboot.

## Start and inspect

In console userland `sh`, Memento's Shell window, or TNT, use:

```text
bg eth
bg wgd --check /mnt/tmp/WGD.CFG
read /mnt/tmp/WGD.LOG
```

If `eth` is already running, leave it running. Wait for `configuration valid` before starting the daemon:

```text
bg wgd /mnt/tmp/WGD.CFG
read /mnt/tmp/WGD.LOG
```

`--check` validates the file without initializing entropy, networking or the tunnel ABI. In the text kernel's rescue shell, `fg wgd --check /mnt/tmp/WGD.CFG` runs the check in the foreground. Use the same path when starting the daemon.

The daemon waits up to ten seconds for `eth` to publish its address. Entropy initialization can also take several seconds. Wait for `listening; public key follows` in the log before probing. This means startup completed; a handshake is still needed to exchange encrypted packets.

Child programs normally print to the kernel console. In graphics mode that output is not visible in Memento's Shell window. The updated userland shell's `bg wgd …` displays a startup log snapshot after 250 ms and prints the diagnostic path. Read `/mnt/tmp/WGD.LOG` again for later events or failures.

The log records startup errors, CPU brand and random-instruction support, entropy selection, session events, and counters refreshed every five seconds: handshakes, received/sent inner packets, drops, replay drops and session state. It logs the public key, but no private keys or packet contents. The log is a bounded snapshot, so older events can be replaced.

Use `ts` to find the PID and `kill <pid>` to stop the daemon. Exit, kill or crash releases tunnel registration and queued frames. Probe ownership is also released when its shell exits.

## Ping and traceroute through subnets

Once startup completes, run these userland shell builtins:

```text
ping 10.4.6.68
traceroute 10.4.6.68
```

They work in console `sh`, Memento's Shell window and TNT. The kernel rescue shell does not implement them; start `fg sh` there first. They require an active `wgd` and a numeric unicast IPv4 destination in `AllowedIPs`, and explicitly select r2's tunnel address as the source. They do not resolve DNS names or probe through the physical Ethernet path.

`ping` sends four ICMP echo requests. `traceroute` sends one ICMP echo probe per hop, increasing TTL from 1 to 30, and displays authenticated time-exceeded, destination-unreachable or echo replies. The first probe allows eight seconds for a handshake; later probes wait two seconds. Only one shell probe client can hold the tunnel reservation at a time.

Every authenticated incoming source must match `AllowedIPs`, including routers answering traceroute. A router outside those prefixes appears as `*`; add its known address or subnet if its reply should be accepted. A configured endpoint, or an endpoint already learned from an authenticated handshake, is needed for r2 to send first.

## Entropy on older CPUs

The daemon checks CPUID before using RDSEED or RDRAND and retries instruction failures. If both are unavailable or exhaust their retries, it uses the pinned upstream Jitterentropy library to collect CPU execution and memory-access timing noise with the high-resolution timestamp counter. This path needs no seed file or configuration option, and is also used if a hardware random source fails during operation.

Jitterentropy applies its upstream conditioning, timer/noise startup tests, and continuous repetition, adaptive-proportion and lag-predictor health tests. The timed code is built at `-O0` without LTO; its memory noise loop has a 2 MiB cap, plus collector state. Startup failure prevents tunnel registration. Runtime failure clears the requested output, detaches the tunnel and stops the daemon.

The log reports the selected source and health-test error codes. RTC time supplies handshake timestamps; it is not an entropy seed. The integration does not substitute uptime, addresses, keys or constants for random input. Passing health tests does not establish the entropy rate of a particular machine, and this integration has no platform entropy assessment or FIPS validation.

The fallback has passed interoperability tests in the graphics kernel under QEMU with a Sandy Bridge CPU model and both random instructions disabled, including hosted-shell ping and traceroute. Actual bare-metal results depend on the machine's timer and noise source; inspect its startup log.

## Routing and limits

`AllowedIPs` serves two purposes: outgoing destination matching and authenticated incoming source checking. The kernel queues frames from the tunnel source address to `wgd` when their destination matches a configured prefix. The daemon's outer UDP goes through the physical NIC, including with `AllowedIPs = 0.0.0.0/0`. A full daemon queue returns `Busy` rather than sending the selected tunnel frame in plaintext.

TCP connections arriving at r2's tunnel address go to the usual bound TCP services. Updated libcr2 preserves that local destination in the socket and replies from it. Ordinary outbound applications must explicitly select the tunnel source address in their network stack; starting `wgd` does not change the physical address or default route.

The daemon supports ICMP for its own tunnel address and probes, TCP, preshared keys, endpoint roaming, replay protection (including keepalives), handshake cookies, bounded handshake work, keepalives, rekeying and expiry. The fixed inner MTU is 1420 bytes. Oversize or fragmented packets are dropped. IPv6, inner UDP, IP fragment reassembly, LAN forwarding, NAT, configuration reload and a `wg` control socket are not implemented.

Physical NIC input with r2's tunnel address as source or destination is dropped to prevent bypassing authenticated input or impersonating queued tunnel output. The kernel holds no WireGuard keys or cryptographic protocol state. This is an experimental port without a security audit.

## Troubleshooting

| Log or symptom | What to check |
|----------------|---------------|
| `configuration missing or exceeds 2048 bytes` | Supply the correct mounted path and check the file's size. |
| `kernel lacks the userspace tunnel ABI; rebuild the kernel` | Boot the rebuilt kernel with ABI 2, alongside the rebuilt daemon. |
| `start eth first and wait for an IPv4 address` | Start `eth` and check DHCP/static configuration, NIC link and gateway. |
| `valid UTC RTC time is required` | Correct the firmware RTC date and UTC time. |
| `tunnel address must differ from the physical IPv4 address` | Give WireGuard a separate tunnel address. |
| `another tunnel is registered or tunnel registration failed` | Use `ts` to check for an existing daemon; only one can register. |
| `no usable entropy source; Jitterentropy failed its startup/health tests` | Read the preceding CPU and Jitterentropy error lines. A failed source prevents startup. |
| `ping/traceroute: start wgd, check AllowedIPs, or wait for another probe to finish` | Wait for the listening message, check the target prefix and any probe running in another shell. |
| Probes time out with no successful handshake | Check keys and optional PSK on both sides, endpoint, UDP reachability and RTC time. |
| Handshake succeeds but subnet probes time out | Check remote forwarding, the return route to r2, firewalls and source prefixes in `AllowedIPs`. |
| TCP service answers over Ethernet but not the tunnel | Relink it with current libcr2 so its replies use the tunnel address; check MTU 1420. |

Host crypto/protocol tests, kernel routing tests, entropy failure tests and QEMU interoperability commands are maintained in the [daemon's development guide](https://github.com/krustowski/rou2exOS-apps/tree/master/c/wgd#verification).
