# Intro

`rou2exOS` (shortened to `r2`) is a DOS-inspired hobby operating system for the x86_64 architecture, written in Rust and x86 assembly. 

It boots via GRUB/Multiboot2, runs in 64-bit long mode with a custom identity-mapped page table, and exposes a kernel shell backed by a preemptive round-robin scheduler. 

The kernel speaks directly to the hardware: 

+ ISA DMA for floppy I/O (FAT12), 
+ ATAPI PIO for CD-ROM (ISO9660), 
+ an RTL8139 NIC for Ethernet/TCP networking, 
+ a VESA framebuffer or VGA text mode for output. 

Userland programs are flat ELF binaries loaded into a fixed region (`0x600_000–0xA00_000`) and call into the kernel via interrupt `0x7F`.

![memento](r2-memento-hello-login.png)

Fig. 1: External userspace program demo called `MEMENTO.ELF`. Login window in the foreground, a wallpaper in the background.

## Blog posts

+ [Original RoureXOS project (krusty.space)](https://krusty.space/projects/rourexos/), June 6, 2024
+ [rou2eXOS Rusted Edition (blog.vxn.dev)](https://blog.vxn.dev/rou2exos-rusted-edition), May 30, 2025
+ [Show HN: A DOS-like hobby OS written in Rust and x86 assembly (news.ycombinator.com)](https://news.ycombinator.com/item?id=44318588), June 19, 2025
+ [rou2exOS: a DOS-like hobby operating system written in Rust (osnews.com)](https://www.osnews.com/story/142612/rou2exos-a-dos-like-hobby-operating-system-written-in-rust/), June 20, 2025
+ [Developer Creates Rust-Based DOS-Like Operating System with Modern Networking Stack (finance.biggo.com)](https://finance.biggo.com/news/202506201922_Rust_DOS-Like_OS), June 20, 2025
+ [rou2exOS - Rust와 x86 어셈블리어로 작성된 Dos-like 취미 OS (news.hada.io)](https://news.hada.io/topic?id=21622), June 24, 2025

## Repositories

+ [rou2exOS kernel](https://github.com/krustowski/rou2exOS)
+ [r2apps](https://github.com/krustowski/r2apps)

---

*Note: Some portions of this documentation have been generated from the original source code by AI.*
