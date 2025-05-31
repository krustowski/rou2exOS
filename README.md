# rou2rexOS Rusted Edition

A second iteration of the RoureXOS operating system, rewritten in Rust.

[Original RoureXOS (a blog post)](https://krusty.space/projects/rourexos/).

[rou2exOS Rusted Edition (a blog post)](https://blog.vxn.dev/rou2exos-rusted-edition)

## How to use (kernel)

```shell
# go to the `kernel` subdirectory
cd kernel

# install Rust and its dependencies
make init

# make sure you have `xorriso`, `net-tools` and `grub2-tools` installed (Linux)
dnf install xorriso net-tools grub2-tools qemu qemu-common qemu-system-x86

# compile the kernel and stage2 bootloader, link it into an ELF binary and bake into an ISO image with GRUB stage1 bootloader
make build

# run the QEMU emulation with ISO image (respectively with additional floppy image attached as well)
make run_iso
make run_iso_floppy
```

