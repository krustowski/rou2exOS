# r2apps and libcr2

An application suite shipped (usually) on floppy disk images (`fat.img`) with the kernel releases.

+ [Link to repository](https://github.com/krustowski/r2apps)

## libcr2

A C/C++ fundamental library implementing the kernel ABI. This library provides a basic set of functions and types that enables a programmer to write programs for the `r2` kernel directly.

Because there is no C runtime version 0 (`crt0`) in the kernel itself, all C/C++ standalone programs must be linked with a special `_crt0.o` object file. This file can be obtained by cloning the `r2apps` repository and by compiling the runtime assembly stub with NASM:

```asm
nasm -f elf64 -o _crt0.o c/libcr2/_crt0.asm
```

As already mentioned, an application targeted for the `r2` kernel needs to be statically linked against `libcr2`, bacause only this library provides the fundamental bindings to system calls implementations.

## Suite

Some of usually shipped applications for `r2` are listed below.

When an app wants to run over Ethernet, the default `ETH` driver must be run beforehand to properly obtain and register an IPv4 address.

| App name | Language | Description |
|----------|----------|-------------|
| `CHAT` | C | A chatroom server over TCP/9000 and HTTP (TCP/8080). Can run on SLIP, or Ethernet. |
| `ETH` | C | An Ethernet networking driver. ARP + ICMP responder and DHCP client. Dafualt `r2` driver for Ethernet. |
| `FSCK` | C | The FAT12 filesystem scanning and diagnostic tool. |
| `GARN` |  C | A very simple HTTP/1.0 server. It enables a file sharing over HTTP. It can be configured using a special configuration file. |
| `GFXTEST` | C | A demo implementation of VGA mode 13h graphical capabilities (video test). |
| `MEMENTO` | C++ | A demo app integrating the Memento GUI framework with `libcr2` into a functional graphical user interface (GUI). |
| `R2SH`/`SH` | C | A userland shell providing a simple command set. |
| `THEM` | C | A 16-bit x86 CPU emulator. Enables running old MS-DOS games and tools. |
| `TNT` | C | A TELNET server and remote shell. The application can run over SLIP, or Ethernet. It exposes port TCP/23 for the host's IP. |