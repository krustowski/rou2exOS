# libcr2 (C)

`libcr2` is the C library for the `r2` kernel: thin wrappers around every syscall, a small `printf`, string and memory helpers, and a TCP/IP stack. It is the oldest part of the SDK and most of the shipped [applications](apps.md) are built on it.

+ [Source (`c/libcr2`)](https://github.com/krustowski/rou2exOS-apps/tree/master/c/libcr2)

| Header | Covers |
|--------|--------|
| `syscall.h` | Syscall numbers (`SyscallNo_T`), the raw `syscall()` entry, one wrapper per syscall, and the structures of the newer calls (`WriteRange_T`, `MemInfo_T`) |
| `types.h` | Structures the kernel reads and writes: `SysInfo_T`, `RTC_T`, `Entry_T`, `VfsDirEntry_T`, `MountInfo_T`, `FsStat_T`, `TaskInfo_T`, `FBInfo_T`, `FsckReport_T`, `MousePacket_T`, … `TaskInfo_T` is 28 bytes and includes the task's last `rip`. |
| `printf.h` | `printf` with a minimal set of conversions |
| `string.h`, `mem.h`, `bytes.h` | `strlen`, `memcmp`, `memcpy`, byte-order helpers |
| `args.h` | Argument parsing helpers |
| `net.h` | SLIP decoding, IPv4/ICMP/TCP parsing, ARP, a DHCP client, and a small TCP socket layer (`bind`, `listen`, `tcp_read`, `tcp_write`, `tcp_close`) |

---

## Building the Library

In the `c/` directory:

```
make libcr2
```

produces the static archive `c/libcr2.a`. Programs link it with `-lcr2`.

## crt0

The kernel has no C runtime, so every program is linked with `_crt0.o`, assembled from `c/libcr2/_crt0.asm`:

```
nasm -f elf64 -o _crt0.o c/libcr2/_crt0.asm
```

`_start` reads `argc` and `argv` from the frame the kernel left on the stack, switches to a private stack in `.bss` (1.5 MiB by default), calls `main(argc, argv)`, and passes its return value to the exit syscall. The stack size can be changed per program with `-DR2_STACK_KB=<n>`: code, data, `.bss` and this stack all share the one 2 MiB frame at `0x600_000`, so a program with large static buffers has to take the room from the stack.

## Building a Program

Each program lives in its own directory under `c/` with a Makefile that compiles freestanding, links `_crt0.o` + objects + `libcr2.a` with the shared `c/linker.ld`:

```make
compile:
	@cd ${BUILD_DIR} && gcc -c -O2 -flto -m64 -static -nostdlib -nostdinc \
		-mno-red-zone -ffreestanding -I .. -I ../../libcr2/ ${SOURCE_FILES}

link:
	@gcc -nostdinc -nostdlib -nostartfiles ${BUILD_DIR}/*.o \
		-L .. -lcr2 -static -Xlinker "-T../linker.ld" -o ${NAME}.elf
```

`c/Makefile.tmpl` holds the same rules for reuse. `-mno-red-zone` is required: the kernel interrupts userland, and an interrupt frame would land in the red zone below `rsp`.

The linker script places `.text` at `0x600000` with `.data` and `.bss` following, in two `PT_LOAD` segments (R+X, R+W).

A minimal program:

```c
#include "syscall.h"

int main(int argc, char **argv) {
    print((const uint8_t *)"Hello, world!\n");
    return 0;
}
```

---

## API at a Glance

| Area | Functions |
|------|-----------|
| Process | `exit` (also `r2_exit`), `run_elf`, `list_tasks`, `kill_task` |
| System | `read_sysinfo`, `write_sysinfo`, `set_user`, `read_rtc`, `get_ticks`, `sleep_ms`, `read_meminfo` |
| Console | `print`, `printf`, `clear_screen` |
| Input | `pipe_subscribe`, `pipe_read`, `pipe_unsubscribe` (keyboard); `pipe_mouse_subscribe`, `pipe_mouse_read`, `pipe_mouse_unsubscribe` |
| Graphics | `get_fb_info`, `write_pixel`, `write_vga`, `blit_buffer`, `blit_buffer_scaled`, `map_vram`, `set_video_mode`, `get_kernel_font`; capture: `capture_framebuffer`, `capture_framebuffer_rgb24_scaled`, and `capture_framebuffer_rgb24_scaled_if_new`, which skips a snapshot the caller already has ([`0x1d` metadata](../abi/syscalls/video_audio.md#metadata-bit-63-of-argument-2)) |
| Audio | `play_freq`, `play_midi_file`, `stop_speaker` |
| Files | `read_file`, `read_file_at`, `write_file`, `write_file_at`, `rename_file`, `delete_file`, `write_subdir`, `chdir` (also `r2_chdir`), `list_dir`, `list_dir_path`, `list_mounts`, `fs_stat` (a mount's size and format, syscall `0x40`; the `FS_TYPE_*` and `FS_FORMAT_*` constants name what it returns), `run_fs_check` |
| Memory | `malloc`, `realloc`, `free` — on the kernel's shared userland heap, its extension included; `shared_heap_contains(ptr, size)` says whether a whole range lies in it without touching it (syscall `0x43`; on older kernels only the original 4 MiB can be checked) |
| Ports, serial | `read_port`, `write_port`, `serial_init`, `serial_read`, `serial_write` |
| IPC | `send_data`, `receive_data`, `receive_data_nb` (syscalls `0x36`, `0x35`) |
| Networking | `new_packet`, `send_packet`, `send_eth_frame`, `net_register`, `net_bind_port`, `net_unbind_port`, `get_net_status`, `get_net_config`, `set_net_config`, `net_*`; sockets: `bind`, `listen`, `tcp_read`, `tcp_write`, `tcp_close` (also `read`, `write`, `close`), `on_tcp_packet` |

For bigger files, prefer `read_file_at` / `write_file_at` (syscalls `0x39` / `0x3a`): `read_file` is never told the size of its buffer, and `write_file` always writes exactly one 512-byte block.

---

## Networking

`net.c` implements, on top of the raw packet syscalls, everything a server needs: SLIP decoding for the serial link, ARP, ICMP echo, a DHCP client and a passive TCP socket pool. The [`ETH`](apps.md) driver uses it to register as the machine's Ethernet driver and obtain an address; [`GARN`](apps.md), [`TNT`](apps.md) and [`CHAT`](apps.md) bind TCP ports on top of it.

By default the Ethernet driver reads frames from the process's kernel queue (syscall `0x35`). A process has only one queue, so a program that runs a second network stack next to libcr2's cannot let both read it: each would take and discard the other's frames. Such a program reads the queue itself and passes libcr2 its frames through `net_set_frame_source(fn)`. The callback fills a buffer with one frame and returns its length, or 0 when no frame is waiting; a blocking call waits for one. Passing `NULL` returns libcr2 to the kernel queue. [Memento](memento.md) does this so that its web browser and its Chat and IRC windows can share one queue. libcr2's frame buffer is 1518 bytes, because the kernel delivers frames with the card's 4-byte CRC still attached.

Current TCP sockets keep the incoming local destination address and use it as the reply source, allowing services to answer connections made to the [WireGuard tunnel address](../networking/wireguard.md). Relink older server binaries to get the fix. New outbound tunnel connections require the network stack to select that source address explicitly; the physical configuration read by `get_net_config` remains the Ethernet configuration.

The kernel-side model (driver registration, port binding, per-tick frame delivery and tunnel routing) is described in [Networking Overview](../networking/overview.md).

---

## With a C Library

libcr2 also works next to the small C library of the [TCC port](tcc.md), which programs compiled on `r2` link against. Its headers notice that library by `<sys/r2libc.h>` on the include path (`R2_LIBC`, in `types.h`) and then leave the C names to it: the integer types, `printf`, `strlen`, `memcpy`, and five calls of libcr2's own, which are known there by other names:

| Name in an app built here | Name next to the C library |
|---------------------------|----------------------------|
| `exit(pid, code)` | `r2_exit(pid, code)` |
| `chdir(path)` | `r2_chdir(path)` |
| `read(sock, buf, max)` | `tcp_read(sock, buf, max)` |
| `write(sock, buf, len)` | `tcp_write(sock, buf, len)` |
| `close(sock)` | `tcp_close(sock)` |

The names on the right exist in every build, so the apps here, built with `-nostdinc`, have both. On `r2`, `tcc -o srv.elf srv.c -lcr2` builds a program on libcr2's TCP/IP stack.

---

## Known Issues

- **`memcpy` takes a `uint16_t` length** and silently truncates copies over 65535 bytes. Programs on the TCC port's C library get that library's `memcpy` instead.

`syscall()` loads the number into both `RAX` (what the kernel reads) and `RDX` (what older kernels read), passes its third argument in `RCX` (read by [`0x1d`](../abi/syscalls/video_audio.md#metadata-bit-63-of-argument-2) with its opt-in bit, and by [`0x44`](../abi/syscalls/port_networking.md#0x44-userspace-ipv4-tunnel), which requires a frame length for operation 3 and zero otherwise), and lists `R9` as clobbered. Earlier versions loaded `RDX` only and left `R9` out of the clobbers; programs built against them should be rebuilt.
