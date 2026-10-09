# TCC (C on r2)

[TinyCC](https://repo.or.cz/tinycc.git) ported to `r2`: the compiler runs on `r2` and builds `r2` programs there, without a host. It ships in the boot image with a small C library, its headers and start-up files, and libcr2.

+ [Source (`c/tcc`)](https://github.com/krustowski/rou2exOS-apps/tree/master/c/tcc)

Upstream tcc is fetched at a pinned commit and left unmodified. The port adds:

| Directory | What it is |
|-----------|------------|
| `libc/` | A small C library on libcr2's syscalls: stdio, the `printf` family (floating point included), strings, `strto*`, `qsort`, time, `setjmp`, and file descriptors over the kernel's named-file calls. tcc is linked against it, and so is every program tcc builds on `r2`. |
| `r2/` | tcc's configuration for `r2`, and a wrapper around its `main` that adds the options every `r2` program needs: `-static -Wl,-Ttext=0x600000 -D__r2__=1` |
| `tests/` | `tcct` (does a compiler's output load and run?), `libct` (does the C library behave like glibc?) and `r2ct` (do the C library and libcr2 go together?), with the `INIT.RC` scripts that run them |
| `run-qemu.sh` | Boots `r2` with files on a scratch floppy, waits for the result files a test writes, and prints them |

---

## Where It Lives

`make build` in `c/tcc` produces `build/sysroot`, and the kernel's `build_iso` copies it into `iso/bin`, `iso/lib` and `iso/include`. On `r2` that is:

| Path | Contents |
|------|----------|
| `/mnt/tar/bin/tcc.elf` | The compiler |
| `/mnt/tar/include` | tcc's own headers and the C library's; libcr2's in `include/r2/` |
| `/mnt/tar/lib` | `crt1.o`, `crti.o`, `crtn.o`, `libc.a`, `libcr2.a`, `libtcc1.a` |

The same files are under `/mnt/iso` when booted from CD. tcc looks in `/mnt/tar` (`r2/config.h`); `-B <dir>` points it at another root laid out the same way.

---

## Using It

Only FAT volumes are writable, so compile where the output can be written: the floppy (`/mnt/fat`) or the RAM disk (`/mnt/tmp`).

```
cd /mnt/fat
fg tcc -o hello.elf hello.c
fg hello
```

The kernel passes a program at most 8 arguments, which is why the options every build needs are in `r2/main.c` rather than on the command line.

libcr2 comes with it: its headers as `<r2/syscall.h>`, `<r2/net.h>` and so on, and its TCP/IP stack as `-lcr2`:

```
fg tcc -o srv.elf srv.c -lcr2
```

libcr2's syscall wrappers are in `libc.a` already. Five libcr2 calls go by other names next to the C library, because C has those names: `r2_exit(pid, code)`, `r2_chdir(path)`, and for sockets `tcp_read`, `tcp_write` and `tcp_close` (see [libcr2: With a C Library](libcr2.md#with-a-c-library)).

TinyCC compiles C, not C++.

### From the Editor

The `r2` port of the Turbo C++ IDE (`tcpp.elf`, Memento's [Editor](memento.md)) compiles with it: **F9** saves the current `.C` file and builds it into an `.ELF` beside it, and **Ctrl+F9** builds and runs it. Diagnostics appear in the Build panel. The compiler runs as a separate task (`tcc.elf --ide 0x<address>`), with the request in a versioned block on the shared user heap (`r2/ide.h`). That entry point uses TinyCC's library API with the same link options as the command line, so source and output paths do not count against the eight arguments. The IDE and the compiler have to be updated together in the boot image.

---

## The C Library

It is the part of C99 that tcc needs, plus what a program reaches for next. On `r2`:

- **A file is a name and an offset.** The kernel keeps no open files: each read or write names the file (syscalls [`0x39` and `0x3a`](../abi/syscalls/filesystem.md)). Names are at most 63 bytes. A new file appears on its first write; mode `w` deletes and recreates it. A file's size is found by probing, as there is no `stat`.
- **`stdout` is line-buffered and `stderr` unbuffered**, both to the console. `exit()` flushes every stream, so programs must start from this library's `crt1.o`, not libcr2's `_crt0.asm`. `stdin` is always empty.
- **`malloc` is the kernel's user heap**, which hands out zeroed memory.
- **`printf` floats are exact to 18 significant digits**; further digits print as 0. `strtod` scales in `long double`, so a constant can, rarely, be off by its last bit.
- **Not there:** `scanf`, the environment (`getenv` returns `NULL`), locales, time zones (the RTC is taken as UTC), `system` and `exec`.

---

## Building and Testing the Port

On the host, in `c/tcc`:

```shell
make build    # clones tinycc, builds a host tcc, then build/sysroot
make test     # tcct, libct and tcc on r2, each in QEMU
```

| Target | What runs on `r2` |
|--------|-------------------|
| `test-tcct` | `tcct`, built by the host tcc against libcr2 alone |
| `test-libct` | `libct`, built by gcc against this C library |
| `test-tcc` | `tcc -o r2ct.elf r2ct.c -lcr2`, `tcc -o libct.elf libct.c`, and both programs |
| `test-image` | The same, on `r2_main/r2.iso` as it is (graphics kernel, tcc as shipped) |
| `libct-host` | (on Linux) `libct` against glibc, the reference for its expected values |

`run-qemu.sh` reads `R2_MAIN` (the kernel checkout, `../../../r2_main` by default), `R2_ISO` (an image to boot as it is), `R2_TAR_ADD` (a directory to add to the boot archive), `R2_WORK` and `R2_TIMEOUT`.

---

## Known Issues

- The host tcc is built without bounds checking and backtraces: their runtime does not build out of tree.
- `tcc -run` is compiled in but untested.
- On the graphics kernel, `sleep_ms` returns after one tick whatever it is asked for (`r2ct` reports it); the text kernel sleeps as long as it should.
