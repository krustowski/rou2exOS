# rou2rexOS Rusted Edition

Second iteration of the RoureXOS operating system, rewritten in Rust.

## test ICMP/SLIP 

Run the kernel in QEMU to get the `pty` number in stdout:

```
make run

char device redirected to /dev/pts/3 (label serial0)
```

Listen for SLIP packets and create a `sl0` interface:

```
sudo slattach -L -p slip -s 115200 /dev/pts/3
sudo ifconfig sl0 192.168.3.1 pointopoint 192.168.3.2 up
```

Catch packets using `tcpdump`:

```
sudo tcpdump -i sl0
```

## UDP Echo server

```
if ipv4_header.protocol == 17 { // UDP
    if let Some((src_port, dst_port, payload)) = net::udp::parse_udp_packet(ip_payload) {
        // Prepare a reply
        let mut udp_buf = [0u8; 512];
        let udp_len = net::udp::create_udp_packet(
            [192, 168, 3, 2],    // our IP
            ipv4_header.source_ip,
            dst_port,            // we swap src/dst ports
            src_port,
            payload,
            &mut udp_buf,
        );

        let mut ipv4_buf = [0u8; 1500];
        let ipv4_len = net::ipv4::create_packet(
            [192, 168, 3, 2],
            ipv4_header.source_ip,
            17, // UDP protocol number
            &udp_buf[..udp_len],
            &mut ipv4_buf,
        );

        net::ipv4::send_packet(&ipv4_buf[..ipv4_len]);
    }
}
```

## linker script (multiboot2)

Used with `multiboot2` for GRUB2.

```json
	"pre-link-args": {
		"ld.lld": [
			"-n",
			"-Tlink.ld"
		]
	},
```
