# SDK

Software development kit.

## libcr2

A fundamental C/C++ library implementing the kernel ABI. This library provides a basic set of functions and types that enables a programmer to write standalone programs for the `r2` kernel directly.

Because there is no C runtime version 0 (`crt0`) in the kernel itself, all C/C++ standalone programs must be linked with a special `_crt0.o` object file. This file can be obtained by cloning the `r2apps` repository and by compiling the runtime assembly stub with NASM:

```asm
nasm -f elf64 -o _crt0.o c/libcr2/_crt0.asm
```

As already mentioned, an application targeted for the `r2` kernel needs to be statically linked against `libcr2`, bacause only this library provides the fundamental bindings to system calls implementations.

The statically linkable archive can be compiled in the `c/` directory of the `r2apps` repository using:

```
make libcr2
```

## r2apps Suite

An application suite shipped (usually) on a floppy disk image (`fat.img`) with the kernel releases (`r2.iso`).

+ [Link to repository](https://github.com/krustowski/r2apps)

Some of usually shipped applications for `r2` are listed below.

When an app wants to run over Ethernet, the default `ETH` driver must be run beforehand to properly obtain and register an IPv4 address.

| App name | Language | Description |
|----------|----------|-------------|
| [`CHAT`](https://github.com/krustowski/r2apps/tree/master/c/chat) | C | A chatroom server over TCP/9000 and HTTP (TCP/8080). Can run on SLIP, or Ethernet. |
| [`ETH`](https://github.com/krustowski/r2apps/tree/master/c/eth) | C | An Ethernet networking driver. ARP + ICMP responder and DHCP client. Dafualt `r2` driver for Ethernet. |
| [`FSCK`](https://github.com/krustowski/r2apps/tree/master/c/fsck) | C | The FAT12 filesystem scanning and diagnostic tool. |
| [`GARN`](https://github.com/krustowski/r2apps/tree/master/c/garn) |  C | A very simple HTTP/1.0 server. It enables a file sharing over HTTP. It can be configured using a special configuration file. |
| [`GFXTEST`](https://github.com/krustowski/r2apps/tree/master/c/gfxtest) | C | A demo implementation of VGA mode 13h graphical capabilities (video test). |
| [`MEMENTO`](https://github.com/krustowski/r2apps/tree/master/cpp/memento-hello) | C++ | A demo app integrating the Memento GUI framework with `libcr2` into a functional graphical user interface (GUI). |
| [`R2SH`/`SH`](https://github.com/krustowski/r2apps/tree/master/c/r2sh) | C | A userland shell providing a simple command set. |
| [`THEM`](https://github.com/krustowski/r2apps/tree/master/c/them) | C | A 16-bit x86 CPU emulator. Enables running old MS-DOS games and tools. |
| [`TNT`](https://github.com/krustowski/r2apps/tree/master/c/tnt) | C | A TELNET server and remote shell. The application can run over SLIP, or Ethernet. It exposes port TCP/23 for the host's IP. |