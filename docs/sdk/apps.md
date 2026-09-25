# Application Suite

The programs built with the [SDK](index.md) and shipped with kernel releases. Source is in the [rou2exOS-apps](https://github.com/krustowski/rou2exOS-apps) repository.

## Where They Live

`make build` in the kernel tree copies the main binaries into `iso/bin/`, so they end up on the ISO at `/mnt/iso/bin` and can be started by name from any directory (`bg eth`, `fg sh`). `make build_floppy` puts data files and demos onto `fat.img`, grouped in directories:

| Floppy directory | Contents |
|------------------|----------|
| `/` | `INIT.RC` |
| `GARN/` | `GARN.CFG`, `INDEX.HTM`, `HELLO.TXT`, `FAVICON.ICO`, `SOCKETS.JSN` |
| `GFX/` | `CUBE.ELF`, `GFXTEST.ELF`, `MEMENTO.ELF` |
| `SLIP/` | `ICMPR.ELF` |
| `SOUND/` | MIDI files for syscall `0x1b` |
| `THEM/` | Real-mode programs for the `THEM` emulator |

Programs that use Ethernet need the [`ETH`](#networking) driver running first, so it can register the NIC and obtain an IPv4 address.

---

## System Tools

| App | Language | Description |
|-----|----------|-------------|
| [`SH`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/r2sh) | C | Userland shell with a simple command set; the intended everyday shell rather than the [kernel shell](../shell.md). With `--host` it runs as the terminal of Memento's Shell window. |
| [`FSCK`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/fsck) | C | FAT12 filesystem scan and diagnostic report (syscall `0x2b`). |
| [`HELLOFS`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/hello-fs) | C | Example of the filesystem syscalls. |
| [`THEM`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/them) | C | 16-bit x86 (8086/80186) emulator with the BIOS and DOS services, PIC, PIT and keyboard a DOS program expects. Runs `.COM` and `MZ` `.EXE` programs (a bare name tries `.COM`, then `.EXE`), e.g. old MS-DOS games from `/mnt/iso/games`. `debug` traces to the screen; `log` writes a periodic record with a heartbeat to `/mnt/fat/THEM.LOG`. |

## Networking

| App | Language | Description |
|-----|----------|-------------|
| [`ETH`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/eth) | C | The default Ethernet driver: registers the RTL8139, answers ARP and ICMP, obtains an address by DHCP (or `--ip <addr>`). |
| [`GARN`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/garn) | C | Small HTTP/1.0 server for sharing files; configured with `--config /mnt/fat/GARN/GARN.CFG`, where `ip` sets a static address (otherwise `ETH`/DHCP manages it). |
| [`TNT`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/tnt) | C | TELNET server and remote shell on TCP/23, over SLIP or Ethernet. |
| [`CHAT`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/chat) | C | Chatroom server on TCP/9000 with an HTTP front end on TCP/8080. |
| [`NSK`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/nsk) | C | Network swiss knife: sweeps a subnet given in CIDR notation and lists live hosts. |
| [`ICMPR`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/icmpresp) | C | ICMP echo responder over SLIP. A Go port lives in `go/icmpresp`. |
| [`DISH`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/dish) | Go | Port of the [dish](https://github.com/thevxn/dish) one-shot monitoring service: HTTP, TCP and ICMP checks, results pushed to HTTP channels. |

## Graphics, Games and Demos

| App | Language | Description |
|-----|----------|-------------|
| [`MEMENTO`](memento.md) | C++ | Windowed desktop on the Memento UI toolkit: file manager, task and memory monitor, shell, clock, chat, IRC, MIDI player, web browser, Snake, Minesweeper, and Turbo C++ in a window. See [Memento (GUI)](memento.md). |
| [`SNAKE`](https://github.com/krustowski/rou2exOS-apps/tree/master/cpp/libc++r2/examples/snake) | C++ | Snake, built on libc++r2; keeps the score on the floppy. |
| [`CUBE`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/cube) | C | Rotating 3D cube in VGA mode 13h. |
| [`GFXTEST`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/gfxtest) | C | VGA mode 13h test. |
| [`GFXDEMO`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/gfxdemo) | Go | Plasma, bouncing balls and kernel-font text through whichever display path the machine has; reports the frame rate. |

## Examples and Tests

| App | Language | Description |
|-----|----------|-------------|
| [`HELLO`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/hello) | Go | Minimal Go program: arguments, sysinfo, RTC. |
| [`ROUTTEST`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/routtest) | Go | Measures what goroutines cost on `r2`. |
| [`hello`](https://github.com/krustowski/rou2exOS-apps/tree/master/cpp/libc++r2/examples/hello) | C++ | Console, containers, filesystem and system info with libc++r2. |
| [`SELFTEST`](https://github.com/krustowski/rou2exOS-apps/tree/master/cpp/libc++r2/tests/target) | C++ | libc++r2 on-target test suite; writes `CXXTEST.TXT`. |
| [`hello-world`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/hello-world) | C | Minimal C program. |
| [`ipc`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/ipc) | C | Message passing between processes (syscalls `0x35`/`0x36`). |
