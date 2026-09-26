# libgor2 (Go)

Go on `r2` is [TinyGo](https://tinygo.org), not the `gc` toolchain. What you get is the whole Go language (slices, maps, strings, interfaces, closures, `defer`, `panic`, goroutines and channels) plus a garbage collector and a useful part of the standard library, in a binary small enough to live in the 2 MiB frame the kernel gives a process.

The Go workspace has three parts:

| Package | Purpose |
|---------|---------|
| [`go/tinygo-r2`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/tinygo-r2) | The TinyGo target: runtime hooks, entry point, memory map |
| [`go/libgor2`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/libgor2) | The syscall binding: one function per syscall, plus the types they read and write |
| [`go/r2net`](https://github.com/krustowski/rou2exOS-apps/tree/master/go/r2net) | The TCP/IP stack: ARP, IPv4, ICMP, UDP, DNS, TCP and an HTTP/1.0 client |

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

The top 256 KiB of the frame is left unused because the kernel's initial-stack table puts slots 8 and 9 at `0x7F0000` and `0x7D0000`, inside the frame. There is no guard page: a stack overflow runs into the top of the heap.

---

## libgor2

```go
import "github.com/krustowski/rou2exOS-apps/go/libgor2"
```

| File | Covers |
|------|--------|
| `syscall.go` | The `int 0x7f` entry, syscall numbers, error codes |
| `types.go` | Kernel structures, with compile-time size assertions |
| `system.go` | Exit, sysinfo, RTC, ticks, sleep, tasks, `Args` |
| `console.go` | Print, clear, flush |
| `fs.go` | Files, directories (`Mkdir`, `RemoveDir`), mounts, `Chdir`, fsck |
| `video.go` | Framebuffer, VGA modes, blitting, the kernel font |
| `audio.go` | Speaker and MIDI |
| `net.go` | Ports, serial, packets, driver registration |
| `input.go` | Keyboard and mouse pipes |
| `mem.go` | The kernel's shared heap (`KMalloc`, `KBytes`) and the memory report (`ReadMemInfo`, syscall `0x3c`) |

### Memory outside the collector

`KMalloc` returns a `uintptr` for a block on the kernel's 4 MiB user heap. The collector does not see the block, so it is never scanned or freed by the collector. Free it with `KFree`; otherwise the kernel frees it when the process exits. Syscalls accept user-heap buffers, and `KBytes` turns a block into a `[]byte` that any call in the package takes. Use it for anything too large for the ~1.5 MiB Go heap:

```go
addr := libgor2.KMalloc(1 << 20)
defer libgor2.KFree(addr)

n, err := libgor2.ReadFileAt("/mnt/fat/BIG.DAT", libgor2.KBytes(addr, 1<<20), 0)
```

The slice is valid only until `KFree` or `KRealloc`, which may move the block. The collector does not scan it, so a Go pointer stored in it does not keep its target alive. The kernel checks that a buffer lies inside a valid region, not that it fits the slice: a slice shorter than what a syscall writes is overrun.

### Errors and structures

A failing syscall returns an `Errno`, which implements `error` (`libgor2.EFileNotFound`, `libgor2.EInvalidInput`, …). Calls that answer with a count or handle (`Ticks`, `ListTasks`, `Run`, `Receive`) return it directly.

`ReadSysInfo` and `ReadMemInfo` return `EBusy` when the kernel was holding a lock; ask again. `Send` always transmits 512 bytes, padding a shorter buffer with zeroes, and `NewPacket` refuses a buffer shorter than 512 bytes. Give `Receive` a 2048-byte buffer, because the kernel copies the whole frame without being told the buffer's size.

Go has no `packed`, so the three ABI structures with a wide field at an odd offset (`RTC.Year`, `VfsDirEntry.Size`, `TaskInfo.RIP`) hold that field as bytes behind an accessor. Each structure's size is asserted at compile time.

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

Two links are supported: `"eth"` (the RTL8139) and `"slip"` (IPv4 over COM1; needs a kernel built without `serial_debug`).

On Ethernet, `Open` checks whether a driver is already registered (syscall `0x37`):

| Finds | Does | What works |
|-------|------|------------|
| No driver | Registers as the global driver | TCP, ICMP, UDP, DNS, and answering ARP for the machine |
| A driver (`ETH`, `GARN`) | Binds the TCP ports it needs | TCP and HTTP only |

The kernel releases the registration when the process exits, is killed or crashes. TCP is a client with one outstanding segment, exponential backoff, no reassembly and an MSS of 1460. The kernel queues up to 64 frames per process, each in its own 2 KiB buffer, so frames are no longer lost when a process reads them late. Each turn of the stack's loop empties the queue and sleeps for a tick only when the queue was empty. An out-of-order segment gets one duplicate ACK, as RFC 5681 specifies. There is no TLS: `Do` refuses `https://` with `ErrTLS`. Everything runs from one goroutine.

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

- A goroutine costs **32 KiB** of stack, committed at `go`; about thirty fit in the heap at once. Use a loop or a small worker pool, not a goroutine per connection, and spawn in batches.
- **The collector does not run on its own.** Long-running programs that create goroutines should call `runtime.GC()` at a quiet point.
- `runtime.NumGoroutine()` always returns 1; `runtime.ReadMemStats` is accurate.

## Gotchas

- **Taking the address of a local costs an allocation.** Passing `uintptr(unsafe.Pointer(&v))` to a syscall makes TinyGo move `v` to the heap on every call. In a loop, hoist the variable to a package-level cell, as `libgor2` does.
- **Mode 13h must be left explicitly** with `SetVideoMode(Mode03Text)`, and `Clear()` (syscall `0x11`) clears only the text writer.
- **A reported framebuffer may be the text buffer.** On the text-mode kernel `GetFBInfo` reports 80×25 at 16 bpp; check for 32 bpp before blitting.
