# Application Suite

The programs built with the [SDK](index.md) and shipped with kernel releases. Source is in the [rou2exOS-apps](https://github.com/krustowski/rou2exOS-apps) repository.

## Where They Live

`make build` in the kernel tree copies the main binaries into `iso/bin/`, so they end up on the ISO at `/mnt/iso/bin`, and in the boot archive at `/mnt/tar/bin` (the only copy reachable when booted from a USB stick), and can be started by name from any directory (`bg eth`, `fg sh`). The ISO also carries data under `/mnt/iso/opt` (Garn's site, Jug's configuration, the Web demo page, Memento's TLS roots, the [bsh](#system-tools) example scripts), and [TCC](tcc.md)'s headers and libraries in `/mnt/iso/include` and `/mnt/iso/lib`. `make build_floppy` puts data files and demos onto `fat.img`, grouped in directories:

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
| [`SH`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/r2sh) | C | Userland shell; the intended everyday shell rather than the [kernel shell](../shell.md). Its commands are [bsh](https://github.com/krustowski/rou2exOS-apps/tree/master/c/bsh)'s, the base shell it shares with `TNT`. With `--host` it runs as the terminal of Memento's Shell window. Runs `.BSH` scripts (a command a line, `$1`–`$9` for arguments): `bsh /mnt/iso/opt/bsh/HELLO.BSH`, or `sh --run <script>`; examples are in [`bsh/`](https://github.com/krustowski/rou2exOS-apps/tree/master/bsh). |
| [`FSCK`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/fsck) | C | FAT12 filesystem scan and diagnostic report (syscall `0x2b`). |
| [`TCC`](tcc.md) | C | TinyCC on `r2`: compiles C programs on the machine itself (`fg tcc -o hello.elf hello.c`), against its own small C library and libcr2. The Editor's F9 builds with it. See [TCC](tcc.md). |
| [`HELLOFS`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/hello-fs) | C | Example of the filesystem syscalls. |
| [`THEM`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/them) | C | 16-bit x86 (8086/80186) emulator with the BIOS and DOS services, PIC, PIT and keyboard a DOS program expects. Runs `.COM` and `MZ` `.EXE` programs (a bare name tries `.COM`, then `.EXE`), e.g. old MS-DOS games from `/mnt/iso/games`. `debug` traces to the screen; `log` writes a periodic record with a heartbeat to `/mnt/fat/THEM.LOG`. |

## Networking

| App | Language | Description |
|-----|----------|-------------|
| [`ETH`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/eth) | C | The default Ethernet driver: registers the NIC (RTL8139, or an Intel e1000/e1000e/PCH card), answers ARP and ICMP, obtains an address by DHCP (or `--ip <addr>`). |
| [`WGD`](../networking/wireguard.md) | C | Standalone WireGuard daemon: one IPv4 peer, up to 16 AllowedIPs prefixes, encrypted shell ping/traceroute and access to r2 TCP services. Runs alongside `ETH`; startup and counters are in `/mnt/tmp/WGD.LOG`. Uses Jitterentropy when CPU random instructions are unavailable. |
| [`GARN`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/garn) | C | Small HTTP/1.0 server for sharing files; configured with `--config /mnt/fat/GARN/GARN.CFG`, where `ip` sets a static address (otherwise `ETH`/DHCP manages it). |
| [`TNT`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/tnt) | C | TELNET server and remote shell on TCP/23, over SLIP or Ethernet, with the same commands as `SH` (bsh) plus `get` and `net`. Once Memento has been given credentials, a new connection has to log in with them; until then it gets a `root` shell straight away. |
| [`CHAT`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/chat) | C | Chatroom server on TCP/9000 with an HTTP front end on TCP/8080. |
| [`NSK`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/nsk) | C | Network swiss knife: sweeps a subnet given in CIDR notation and lists live hosts. |
| [`ICMPR`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/icmpresp) | C | ICMP echo responder over SLIP. A Go port lives in `go/icmpresp`. |
| [`DISH`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/dish) | Go | Port of the [dish](https://github.com/thevxn/dish) one-shot monitoring service: HTTP, TCP and ICMP checks, results pushed to HTTP channels. |
| [`STREAMD`](https://github.com/krustowski/rou2exOS-apps/tree/master/c/streamd) | C | Streams the screen, Memento windows included, as 640×480 MJPEG at up to 30 FPS on `http://<ip>:8080/stream` (a browser or an OBS Browser Source), one client at a time. Skips frames that have not changed (syscall [`0x1d` metadata](../abi/syscalls/video_audio.md#metadata-bit-63-of-argument-2)); diagnostics in `/mnt/tmp/STREAMD.LOG`. |

## Graphics, Games and Demos

| App | Language | Description |
|-----|----------|-------------|
| [`MEMENTO`](memento.md) | C++ | Windowed desktop on the Memento UI toolkit: file manager, task and memory monitor, shell, clock, chat, IRC, MIDI player, web browser, Snake, Minesweeper, and Turbo C++ in a window. See [Memento (GUI)](memento.md). |
| [`JUG`](jug.md) | C++ | Program manager with a hosted Memento window and console commands: CDN catalog, verified ELF downloads, checksum registry, updates and instance restarts. |
| [`R2WEB`](https://github.com/krustowski/rou2exOS-apps/tree/master/cpp/r2web) | C++ | The browser behind Memento's Web window, one process per window: HTTP/1.1 and HTTPS (BearSSL), HTML/CSS layout, forms, pictures and a little ES5 JavaScript (MuJS). See [Memento](memento.md#notes-on-individual-windows). |
| [`MPEGPLAY`](https://github.com/krustowski/rou2exOS-apps/tree/master/cpp/mpegplay) | C++ | MPEG-1 video with MP2 sound through HD Audio, from files in `/mnt/tar/video` or HLS streams: in Memento's Video window, or full screen in true colour on the graphics kernel (F). |
| [`TCPP`](memento.md#notes-on-individual-windows) | C++ | Turbo C++ 23, the text-mode IDE, in Memento's Editor window or full screen; builds and runs `.C` files with [TCC](tcc.md). |
| [`SPOTIFY`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/spotify) | Go | Spotify client hosted in Memento (experimental): playlists from the Web API, and Premium tracks streamed, decrypted and decoded (Ogg Vorbis) to HD Audio on `r2`, over r2net and r2tls. Offers generated test tones without an account. |
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
