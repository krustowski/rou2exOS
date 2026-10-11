# Kernel Internals

The kernel boots through GRUB and Multiboot2 into x86-64 long mode. It schedules
ring-0 tasks and ring-3 ELF programs, dispatches syscalls through `int 0x7f`, and
provides hardware access and filesystem mounts. TCP/IP stacks, the desktop and
application runtimes live in userland.

Start with [Boot and Initialization](init/overview.md), then follow the subsystem
that owns the behavior you are investigating. For application development, use
the [SDK](sdk/index.md) and [ABI Specification](abi/syscall_specification.md).

| Subsystem | Guides | Source directory |
|-----------|--------|------------------|
| Boot and initialization | [Boot sequence](init/overview.md), [Subsystem setup](init/subsystems.md), [Build and Run](build.md#boot-flow) | `src/init/`, `iso/boot/boot.asm` |
| Tasks and IPC | [Scheduler](multitasking/scheduler.md), [Processes](multitasking/process.md), [IPC](multitasking/IPC.md) | `src/task/` |
| Memory | [Memory map and paging](memory/overview.md), [Allocators](memory/allocators.md) | `src/mem/`, `linker.ld` |
| Filesystems | [VFS, RAM disk and boot archive](filesystem/overview.md), [FAT12/FAT16](filesystem/fat12.md), [ISO9660](filesystem/iso9660.md) | `src/fs/` |
| Networking | [Routing and delivery](networking/overview.md), [NICs and serial](networking/hardware.md), [Protocol helpers](networking/protocols.md), [WireGuard](networking/wireguard.md) | `src/net/` |
| Video | [Framebuffer and capture](video/overview.md), [VGA](video/vga.md) | `src/video/` |
| Audio | [HD Audio PCM](audio/hda.md), [Speaker and MIDI syscalls](abi/syscalls/video_audio.md#0x1a-play-frequency) | `src/audio/` |
| Input | [Input routing](input/overview.md), [Keyboard](input/keyboard.md), [Mouse](input/mouse.md), [USB firmware handoff](build.md#taking-usb-from-the-firmware) | `src/input/`, `src/usb/` |

## Current Interfaces

There are 32 scheduler slots. Each ELF process has a private 2 MiB image and a
slot-specific initial stack; the shared user heap can grow beyond its initial
4 MiB. The writable RAM disk at `/mnt/tmp` and the read-only boot archive at
`/mnt/tar` allow application loading on machines without a floppy or CD driver.

The graphics session's `init_rc` task supervises Memento. A cooperative
[desktop relaunch](abi/syscalls/sysinfo_mem_mgmt.md#0x42-coordinated-desktop-relaunch)
replaces it after a Jug update while keeping the RAM disk and background services.
An ordinary session exit retains the reboot policy described in
[Build and Run](build.md#graphics-session).

Networking supports RTL8139 and selected Intel NICs, loopback, and a userland
WireGuard daemon. [Network monitoring](networking/overview.md#network-monitoring)
exposes physical NIC traffic totals and TCP port owners through syscalls `0x45`
and `0x46`.

## Debugging and Validation

Use the text kernel's [rescue shell](shell.md) for task, heap, mount, NIC, ACPI
and USB diagnostics. On either video path, `/mnt/tmp/KERNDBG.LOG` contains the
[boot debug log](init/overview.md#debug-log). See [Tests](build.md#tests) for host
unit tests, publisher tests and the QEMU kernel self-test.

Related documentation: [Documentation home](index.md), [SDK Overview](sdk/index.md), [ABI Specification](abi/syscall_specification.md).
