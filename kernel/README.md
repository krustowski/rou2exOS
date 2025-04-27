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
