# libgor2 (Go)

Go on `r2` is [TinyGo](https://tinygo.org), not the `gc` toolchain. What you get is the whole Go language (slices, maps, strings, interfaces, closures, `defer`, `panic`, goroutines and channels) plus a garbage collector and a useful part of the standard library, in a binary small enough to live in the 2 MiB frame the kernel gives a process.

The Go workspace has these parts:

| Package | Purpose |
|---------|---------|
| [`go/tinygo-r2`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/tinygo-r2) | The TinyGo target: runtime hooks, entry point, memory map |
| [`go/libgor2`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/libgor2) | The syscall binding: one function per syscall, plus the types they read and write |
| [`go/r2net`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/r2net) | The TCP/IP stack: ARP, IPv4, ICMP, UDP, DNS, TCP and an HTTP/1.0 client |
| [`go/r2tls`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/r2tls) | Certificate-verified HTTPS: Memento's portable BearSSL, through Cgo, over r2net's TCP connections |
| [`go/libgor2/memento`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/libgor2/memento) | The window bridge for a Go program hosted in [Memento](memento.md): launch checks, commands, double-buffered snapshots, heartbeat and shutdown, with the matching C++ side in `host.hpp` |

---

## Quick Start

The only host dependency is Docker.

```
cd go/tinygo-r2 && make image     # once; builds tinygo-r2:0.38.0
cd ../hello && make               # produces hello.elf
```

An application needs a two-line Makefile; the linker script and target definition belong to the target and are shared by every Go program:

```make
NAME := hello

include ../Makefile.tmpl
```

```go
package main

import (
	"fmt"

	"github.com/krustowski/rou2exOS-apps/go/libgor2"
)

func main() {
	fmt.Printf("Hello from Go on r2!\n")

	var info libgor2.SysInfo
	if err := libgor2.ReadSysInfo(&info); err != nil {
		fmt.Printf("sysinfo: %v\n", err)
		return
	}
	fmt.Printf("uptime: %d s\n", info.Uptime)
}
```

Builds use `-target=r2 -opt=z -no-debug`. A `println` hello world is about 10 KiB; with `fmt` it is about 110 KiB.

---

## The TinyGo Target (`tinygo-r2`)

TinyGo reads its standard library and target definitions from `TINYGOROOT` at compile time, so the target is five files dropped into the stock image, with no LLVM rebuild:

| File | What it is |
|------|------------|
| `r2.json` | Baremetal amd64, cooperative scheduler (`-scheduler=tasks`), conservative GC |
| `r2.ld` | The memory map |
| `r2.S` | `_start`, the `int 0x7f` trampoline, the argv stash |
| `runtime_r2.go` | `putchar`, `ticks`, `sleepTicks`, `exit` |
| `interrupt_r2.go` | No-op interrupt shims (ring 3 has nothing to disable) |

The runtime maps onto the ABI as follows:

| Runtime needs | Syscall |
|---------------|---------|
| `putchar` | `0x10`, buffered a line at a time |
| `ticks`, `nanotime`, `time.Now` | `0x04` (milliseconds since boot) |
| `sleepTicks` | `0x05` |
| `exit`, `abort` | `0x00` |
| heap | none — the linker script provides it |

Console output is flushed at each newline and on exit; `libgor2.Flush()` forces a partial line out.

### Memory

```
0x600000  .text / .rodata / .data / .bss
          _heap_start ... _heap_end      the collector's arena, ~1.5 MiB
0x7A0000  stack, 128 KiB, growing down
0x7C0000  left alone
0x800000  end of the frame
```

The top 256 KiB of the frame is left unused because the kernel's initial-stack table used to put slots 8 and 9 at `0x7F0000` and `0x7D0000`, inside the frame. Since the move to 32 slots no stack is there, so the room could be taken back. There is no guard page: a stack overflow runs into the top of the heap.

---

## libgor2

```go
import "github.com/krustowski/rou2exOS-apps/go/libgor2"
```

| File | Covers |
|------|--------|
| `syscall.go` | Syscall numbers and error codes; `Syscall` (two arguments, `RCX` cleared) |
| `abi.go` | The TinyGo assembly and runtime hooks behind them, including `Syscall3`, which passes a third argument in `RCX` |
| `types.go` | Kernel structures, with compile-time size assertions |
| `system.go` | Exit, sysinfo and the system user, RTC, ticks, sleep, tasks, command lines, desktop relaunch, power, `Args` |
| `console.go` | Print, clear, flush |
| `fs.go` | Files, directories (`Mkdir`, `RemoveDir`), mounts (`ListMounts`, and `FsUsage` for a mount's size, free space and format, syscall `0x40`; the `Fs*` and `Format*` constants in `types.go`, `FsMemDisk` and `FormatFat16` among them), `Chdir`, fsck |
| `video.go` | Framebuffer, VGA modes, blitting, the kernel font, screen capture (with metadata) |
| `audio.go` | Speaker and MIDI; HD Audio PCM (`AudioOpen`, `AudioWrite`, `AudioQueued`, pause, resume, close; syscall `0x3f`) |
| `net.go` | Ports, serial, packets, driver registration |
| `input.go` | Keyboard and mouse pipes |
| `mem.go` | The kernel's shared heap (`KMalloc`, `KBytes`) and the memory report (`ReadMemInfo`, syscall `0x3c`) |

### Memory outside the collector

`KMalloc` returns a `uintptr` for a block on the kernel's shared user heap (4 MiB at `0xC00000`, and its extension once that is full). The collector does not see the block, so it is never scanned or freed by the collector. Free it with `KFree`; otherwise the kernel frees it when the process exits. Syscalls accept user-heap buffers, and `KBytes` turns a block into a `[]byte` that any call in the package takes. Use it for anything too large for the ~1.5 MiB Go heap:

```go
addr := libgor2.KMalloc(1 << 20)
defer libgor2.KFree(addr)

n, err := libgor2.ReadFileAt("/mnt/fat/BIG.DAT", libgor2.KBytes(addr, 1<<20), 0)
```

The slice is valid only until `KFree` or `KRealloc`, which may move the block. The collector does not scan it, so a Go pointer stored in it does not keep its target alive. The kernel checks that a buffer lies inside a valid region, not that it fits the slice: a slice shorter than what a syscall writes is overrun.

### A larger Go heap (`r2largeheap`)

A program built with `-tags=r2largeheap` takes its collector arena from the user heap (syscall `0x0a`) before the first Go allocation: 8 MiB, or 4 MiB or 2 MiB if that much is not free, and the arena in the frame if none of it is. Every Go object and goroutine stack lives there; globals and the system stack stay in the frame. The arena never moves, and the kernel frees it when the process exits or faults. It needs a kernel with the user-heap extension. Spotify is built with it. Rebuild the `tinygo-r2` image before using the tag.

### Errors and structures

A failing syscall returns an `Errno`, which implements `error` (`libgor2.EFileNotFound`, `libgor2.EInvalidInput`, …). Calls that answer with a count or handle (`Ticks`, `ListTasks`, `Run`, `Receive`) return it directly.

`ReadSysInfo` and `ReadMemInfo` return `EBusy` when the kernel was holding a lock; ask again. `Send` always transmits 512 bytes, padding a shorter buffer with zeroes, and `NewPacket` refuses a buffer shorter than 512 bytes. Give `Receive` a 2048-byte buffer, because the kernel copies the whole frame without being told the buffer's size.

Go has no `packed`, so the three ABI structures with a wide field at an odd offset (`RTC.Year`, `VfsDirEntry.Size`, `TaskInfo.RIP`) hold that field as bytes behind an accessor. Each structure's size is asserted at compile time.

### Process and system controls

- `CommandLine(pid)` reads up to 128 bytes of the command a process was started with (syscall `0x41`); the result need not end in a NUL.
- `RequestDesktopRelaunch(pid)` asks a registered Memento, running under the graphics session's supervisor, to close and start again (`0x42`). Only that Memento can call `RegisterDesktopRelaunch` and `DesktopRelaunchPending`; anyone else gets `ENotImplemented`.
- `Reboot` and `PowerOff` (`0x3e`) do not return when they succeed.
- `SetUser` changes the system user to one printable ASCII word; `WriteNetConfig` publishes the network configuration, from the global driver only (`0x3d`).
- The file calls take the floppy, the writable RAM disk (`/mnt/tmp`), the read-only ISO and tar volumes, and mount directories such as `/` and `/mnt`.

### Capture metadata

`CaptureFramebufferRGB24ScaledIfNew` is syscall `0x1d` with its [metadata](../abi/syscalls/video_audio.md#metadata-bit-63-of-argument-2), through `Syscall3`. It takes a `*FBCaptureInfo`: leave `FrameID` at zero to always copy, or pass the last ID accepted to get `EUnchanged`, with the pixels left as they were, while the 640×480 snapshot has not changed. On the way out the struct names the frame and its time in milliseconds, and `Flags & FBCaptureInfoSnapshot` says it came from a stable snapshot in RAM. An older kernel does an ordinary capture and leaves the flags and timestamp at zero.

### Testing the binding

`make test` in `go/libgor2` runs host tests of the ABI and of networking with stock Go; the `r2abimock` build tag replaces the assembly and runtime hooks on the host only, and an unexpected syscall panics. `libgor2/tests/native`, built as `ABI.ELF` and run from `INIT.RC` with `fg ABI --smoke`, checks the binding on a real kernel: argv, sysinfo and user changes, command lines, the error returns of the desktop and power calls, VFS mount directories, RAM-disk I/O, pings to the machine through another process's driver, registration cleanup and port binding, and on a 32 bpp boot the capture metadata through the real three-argument trampoline. It prints `GO ABI CHECK PASS` to QEMU's debug console (`-debugcon file:abi.log -global isa-debugcon.iobase=0xe9`) and, with `-device isa-debug-exit,iobase=0xf4,iosize=0x04`, exits QEMU with code 33.

---

## r2net

```go
stack, err := r2net.Open(r2net.Options{
	Link:    "eth",
	LocalIP: r2net.IP{10, 3, 4, 2},
	Gateway: r2net.IP{10, 3, 4, 1},
	DNS:     r2net.IP{10, 3, 4, 1},
})
if err != nil {
	return err
}
defer stack.Close()

res, err := stack.Get("http://api.example.com/health", nil, 10*time.Second)
```

Two links are supported: `"eth"` (the RTL8139 or the E1000, through the kernel's driver registration) and `"slip"` (IPv4 over COM1; needs a kernel built without `serial_debug`).

On Ethernet, `Open` checks whether a driver is already registered (syscall `0x37`):

| Finds | Does | What works |
|-------|------|------------|
| No driver | Registers as the global driver | TCP, ICMP, UDP, DNS, and answering ARP for the machine |
| A driver (`ETH`, `GARN`) | Binds the TCP ports it needs | TCP and HTTP only |

The kernel releases the registration and the TCP port bindings when the process exits, is killed or crashes; `Stack.Close` releases a connection's bindings sooner.

Traffic to `127.0.0.0/8` or to the machine's own address goes through the kernel's [loopback device](../networking/overview.md), so `Ping` to them works even while another process holds the driver. Local TCP still needs a bound destination port, and remote UDP and DNS still need the driver. TCP is a client with one outstanding segment, exponential backoff, no reassembly and an MSS of 1460. The kernel queues up to 64 frames per process, each in its own 2 KiB buffer, so frames are no longer lost when a process reads them late. Each turn of the stack's loop empties the queue and sleeps for a tick only when the queue was empty. An out-of-order segment gets one duplicate ACK, as RFC 5681 specifies. r2net itself speaks cleartext HTTP only (`Do` refuses `https://` with `ErrTLS`); certificate-verified HTTPS is [`r2tls`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/r2tls), BearSSL over r2net's connections, which the Spotify client uses. The stack belongs to one goroutine, but its TCP waits yield to the Go scheduler, so an audio or UI goroutine keeps running while a request is blocked; `Options.Check` can cancel a pending operation.

---

## What Works

Verified on the kernel under QEMU:

- `fmt` (including `%v`, `%q` and floating point), `strconv`, `strings`, `bytes`, `sort`, `errors`, `math`, `unicode/utf8`, `sync`, `encoding/json`, `flag`
- goroutines, channels, `select`, maps, slices
- `time.Now`, `time.Sleep`, `time.Since`
- graphics: VGA mode 13h at 134 fps, scaled VESA blit at 26 fps (`gfxdemo`)
- networking over both links: ICMP, DNS, TCP, HTTP/1.0 (`dish`)

## What Does Not

- **Threads.** The scheduler is cooperative: a goroutine yields at channel operations, `time.Sleep` and `runtime.Gosched()` only. The kernel still preempts the process as a whole.
- **`os` and `net`.** There are no file descriptors or sockets; use `libgor2` and `r2net`.
- **`libgor2.SleepMS` in a program with goroutines** parks the whole process. Use `time.Sleep`.

## Goroutine Costs

Measured by `routtest`:

- A goroutine costs **16 KiB** of stack (`default-stack-size` in `tinygo-r2/r2.json`), committed at `go`; about eighty fit in the default heap at once. The deepest goroutine `routtest` runs writes 1.4 KiB, and the whole of `dish` in one goroutine 7 KiB. Spawn in batches: what counts is how many are outstanding before one is scheduled.
- A program can choose smaller stacks with `-stack-size=8KB` in its `TINYGO_FLAGS`. Check `libgor2.ReadStackStats().Peak` after a representative run first: there is no guard page, and an overflow is caught only by a canary when the goroutine next yields.
- **Finished goroutines hand their stacks to the next `go`.** The collector does not run on its own, so the runtime keeps up to eight finished stacks for reuse (`libgor2.SetStackCache`, up to 32), and a loop of short-lived goroutines needs no collection. A burst wider than the cache leaves the rest to `runtime.GC()`.
- `runtime.NumGoroutine()` always returns 1; `runtime.ReadMemStats` is accurate.

## Gotchas

- **Taking the address of a local costs an allocation.** Passing `uintptr(unsafe.Pointer(&v))` to a syscall makes TinyGo move `v` to the heap on every call. In a loop, hoist the variable to a package-level cell, as `libgor2` does.
- **Mode 13h must be left explicitly** with `SetVideoMode(Mode03Text)`, and `Clear()` (syscall `0x11`) clears only the text writer.
- **A reported framebuffer may be the text buffer.** On the text-mode kernel `GetFBInfo` reports 80×25 at 16 bpp; check for 32 bpp before blitting.
