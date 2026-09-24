# libc++r2 (C++)

`libc++r2` is a C++ runtime and standard library for the `r2` kernel, in the same spirit as [libcr2](libcr2.md): no host libc, no libstdc++, nothing underneath but the kernel ABI. It covers the whole userland surface of the kernel plus the parts of the language runtime that have to exist before C++ can run at all: global constructors, `operator new`, the Itanium ABI hooks, `memcpy` and friends.

+ [Source (`cpp/libc++r2`)](https://github.com/krustowski/rou2exOS-apps/tree/master/cpp/libc++r2)

C++23 by default; C++17 on request (`make STD=c++17`).

```cpp
#include <r2.hpp>

int main(int argc, char **argv) {
    r2::vector<r2::string_view> args;
    for (int i = 0; i < argc; i++)
        (void)args.push_back(r2::arg(i));

    r2::sort(args.begin(), args.end());
    r2::println("hello from C++ on r2, ", args.size(), " arguments");
    return 0;
}
```

---

## Building

```
cd cpp/libc++r2
make            # libc++r2.a, libc++r2compat.a, _crt0.o
make check      # host-side tests
make examples   # examples/hello, examples/gfxdemo, examples/snake
```

An application needs a two-line Makefile:

```make
NAME := hello
include ../../Makefile.tmpl
```

| Target | Result |
|--------|--------|
| `make build` | `hello.elf` and `hello.bin` |
| `make install` | copies `HELLO.ELF` onto `fat.img` (name upper-cased, cut to 8 characters) |
| `make clean` | removes the build output |

Variables that can be set before the `include`: `SOURCES` (default: every `.cpp` beside the Makefile), `LIBCXXR2` (library root), `EXTRA_LIBS` (additional archives, e.g. `libcr2.a`), `FLOPPY` (image for `make install`), `STD`.

Everything is compiled `-ffreestanding -nostdinc -nostdinc++ -fno-exceptions -fno-rtti -mno-red-zone -fno-pie` and linked with `--gc-sections`, so a program that includes the whole library still comes out at a few tens of kilobytes.

Copy `examples/hello/.clangd` next to a new program too: without it clangd parses with the host's default flags and cannot find `<r2.hpp>`.

---

## What Is in It

| Header | Provides |
|--------|----------|
| `r2/vector.hpp`, `r2/string.hpp`, `r2/string_view.hpp`, `r2/array.hpp`, `r2/span.hpp`, `r2/optional.hpp`, `r2/function.hpp` | Containers |
| `r2/algorithm.hpp`, `r2/utility.hpp`, `r2/type_traits.hpp` | `sort` (introsort), `find`, `lower_bound`, `move`, `forward`, `pair`, traits |
| `r2/new.hpp`, `r2/memory.hpp` | `operator new`/`delete`, `unique_ptr`, `construct_at`, uninitialised algorithms |
| `r2/expected.hpp` | `expected<T, E>` / `unexpected<E>` with the C++23 monadic operations |
| `r2/coroutine.hpp` | `coroutine_handle` and friends, plus `generator<T>` |
| `r2/compare.hpp`, `r2/concepts.hpp`, `r2/tuple.hpp`, `r2/initializer_list.hpp` | What `<=>`, concepts, structured bindings and brace-init need from `std::` |
| `r2/io.hpp` | `print`, `println`, `printf("{}")`, `format`, `concat` — type-safe, no varargs |
| `r2/heap.hpp` | The arena allocator and its statistics |
| `r2/syscall.hpp` | The raw ABI: syscall numbers, kernel structures, `raw_syscall` |
| `r2/fs.hpp` | Files and directories |
| `r2/gfx.hpp` | `Canvas`, `Font`, the VESA framebuffer, VGA mode 13h |
| `r2/input.hpp` | Keyboard and mouse |
| `r2/time.hpp` | Ticks, sleep, the RTC, `Stopwatch`, `FrameTimer` |
| `r2/process.hpp` | Arguments, `exit`, sysinfo, the task table, `spawn` |
| `r2/net.hpp` | Addresses, byte order, frames, port binding |
| `r2/audio.hpp` | PC speaker |
| `r2/math.hpp` | `sqrt`, `sin`, `cos`, `floor`, `fmod`, … (there is no libm) |
| `r2/panic.hpp`, `r2/source_location.hpp` | `panic`, `R2_ASSERT` |

`r2.hpp` includes everything. The library's vocabulary lives in namespace `r2::`; only the names the language itself requires are placed in `std::`.

### No exceptions, so failure is in the return value

Every operation that can fail to allocate says so (`[[nodiscard]]`), and `operator new` returns `nullptr` instead of throwing:

```cpp
r2::vector<int> v;
if (!v.reserve(1000)) { r2::println("out of arena"); return 1; }

auto file = r2::fs::read_text("/README.TXT");
if (!file) { r2::println("no such file"); return 1; }
```

`expected<T, E>` lets a function say *what* failed and lets callers chain with `transform`, `and_then` and `value_or`. User types become printable by declaring `format_value(W &, const T &)` in their namespace.

---

## Memory Map

Everything libc++r2 sets up stays inside the private 2 MiB frame:

```
0x600_000  +---------------------------+
           |  .text  .rodata  .data    |   the program
           +---------------------------+
           |  stack     512 KiB        |   _crt0.asm, STACK_BYTES
           +---------------------------+
           |  heap arena 512 KiB       |   heap_arena.cpp, ARENA_BYTES
0x800_000  +---------------------------+   <-- stay below this line
```

Both sizes are build-time settings (`make ARENA_BYTES=262144 STACK_BYTES=262144`), and one application can replace the arena without rebuilding the library:

```cpp
R2_HEAP_ARENA(1024 * 1024)      // at file scope, in exactly one .cpp
```

The heap is an arena in `.bss`, not the kernel heap (syscall `0x0a`), because only memory inside `0x600_000–0xA00_000` can be passed back to a syscall. The arena is a first-fit free list with boundary tags that coalesces on free; `r2::heap::stats()` reports use, free space and the largest block available. The kernel heap is still reachable via `r2::heap::kernel_allocate()` for scratch buffers that never cross the ABI.

---

## Startup

`_crt0.asm` reads the argv frame, switches to the private stack and calls `__r2_start`, which:

1. brings up the heap,
2. runs `.init_array` (global constructors),
3. calls `main` (either `int main()` or `int main(int, char **)`),
4. flushes the console, runs `at_exit` handlers, static destructors and `.fini_array`, then makes the exit syscall.

## Syscalls

`r2::raw_syscall` loads the syscall number into both `RAX` and `RDX` and declares `R9` clobbered, which matches what the kernel's entry stub actually does (see [Syscall Specification](../abi/syscall_specification.md)).

---

## Graphics

Two display paths, chosen at run time:

- **VESA framebuffer.** `Display::open()` describes it and `present()` sends a `Canvas`. A full-screen 1024×768×32 buffer is 3 MiB and does not fit in a process, so draw into a small canvas (320×200 is 256 KiB) and let the kernel scale it with syscall `0x17`.
- **VGA mode 13h.** `Vga13::open()` maps VGA RAM into the process (syscall `0x14`) and switches the mode; the destructor restores text mode.

On a text-mode boot `get_fb_info` still succeeds and describes the 80×25 text buffer, so `Display::open()` additionally requires 32 bpp and at least 320×200 and returns `nullopt` otherwise. Text is drawn with the kernel's PSF font via `Font::kernel()`.

---

## Mixing With Other Libraries

**libcr2**, e.g. for its TCP stack:

```make
NAME       := myserver
EXTRA_LIBS := $(abspath ../../../c/libcr2.a)
include ../../Makefile.tmpl
```

Put `libc++r2.a` first (both define `memcpy`; `libcr2`'s truncates at 64 KiB). Memory from `libcr2`'s `malloc` lives on the kernel heap and cannot be passed back to a syscall.

**Host libstdc++** (the `memento-hello` case): link `libc++r2compat.a` after `-lstdc++`. It supplies the glibc symbols libstdc++ references but never calls on this target, `malloc`/`free` onto the arena, and `_Unwind_*` stubs.

```
g++ ... $(OBJS) libc++r2.a -lstdc++ libc++r2compat.a -lgcc ...
```

---

## Tests

- `make check` runs host-side suites natively: containers, string, `sort`, number formatting and the allocator (built as C++17), the arena override, and the C++20/23 facilities (built as C++23).
- `tests/target/` builds `SELFTEST.ELF`, which runs on `r2` and writes its result to `CXXTEST.TXT` on the floppy.

## Known Rough Edges

- `sleep()` is best-effort; use `ticks()` or `FrameTimer` for timing.
- `fs::write` writes a single 512-byte sector (syscall `0x21`), with no append.
- `fs::size_of()` lists the directory, since there is no stat syscall.
- `sin`, `cos` and `atan` are approximations.
- No `shared_ptr`, no `map`, no iostreams.
