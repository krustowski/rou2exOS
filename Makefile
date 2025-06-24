init:
	@ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@. "${HOME}/.cargo/env" && \
		rustup install nightly && \
		rustup default nightly && \
		rustup target add x86_64-unknown-none && \
		rustup component add rust-src llvm-tools-preview && \
		cargo install bootimage

#
#  BUILD
#

build: compile_kernel nasm link build_iso 

#@cargo rustc --release --target x86_64-r2.json -- -C relocation-model=static --emit=obj
compile_kernel:
	@cargo rustc \
		--release \
		-Z build-std=core,compiler_builtins \
		--target x86_64-r2.json \
		-- --emit=obj

nasm:
	@nasm \
		-f elf64 \
		-o iso/boot/boot.o \
		iso/boot/boot.asm

KERNEL_OBJ := $(shell ls -t target/x86_64-r2/release/deps/kernel-*.o | head -1)

link:
	@ld.lld \
		--verbose \
		-T linker.ld \
		-n \
		--gc-sections \
		-o iso/boot/kernel_graphics.elf \
		${KERNEL_OBJ} iso/boot/boot.o

build_iso:
	@grub2-mkrescue \
		-o r2.iso iso/ \
		--modules="multiboot2 vbe video video_bochs video_cirrus gfxterm all_video"

build_floppy:
	@dd \
		if=/dev/zero \
		of=fat.img \
		bs=512 \
		count=2880
	@mkfs.fat \
		-F 12 \
		fat.img
	@echo "Hello from floppy!" > /tmp/hello.txt
	@mcopy \
		-i fat.img /tmp/hello.txt ::HELLO.TXT

#
#  RUN
#

run:
	@qemu-system-x86_64 \
		-serial pty \
		-drive format=raw,file=target/x86_64-r2/debug/bootimage-x86_64-r2.bin

run_iso: 
	@qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-vga std \
		-cdrom r2.iso \
		-serial pty

run_iso_usb: 
	@qemu-system-x86_64 \
		-m 2G \
		-vga std \
		-hdb /dev/sdb \
		-serial pty

run_iso_net: 
	@qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-vga std \
		-cdrom r2.iso \
		-netdev tap,id=net0,ifname=tap0,script=no,downscript=no \
		-device rtl8139,netdev=net0 \
		-serial pty

PTY_NUMBER ?= pty
run_iso_pty: 
	@qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-vga std \
		-cdrom r2.iso \
		-serial ${PTY_NUMBER}

run_iso_floppy: build_floppy
	@qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-vga std \
		-cdrom r2.iso \
		-fda fat.img \
		-serial pty

run_iso_floppy_drive: 
	@sudo qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-vga std \
		-cdrom r2.iso \
		-serial pty \
		-blockdev host_device,node-name=floppy1,filename=/dev/sda \
		-device floppy,drive=floppy1

run_iso_debug: 
	@qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-cdrom r2.iso \
		-fda fat.img \
		-d int,cpu_reset,page \
		-no-reboot \
		-no-shutdown \
		-serial stdio

#
#  HELPERS
#

clean:
	@cargo clean

clippy:
	@cargo clippy \
		--release \
		--target x86_64-r2.json \
		--no-default-features \
		-- -D warnings


ifeq (${SONAR_HOST_URL}${SONAR_TOKEN},)
sonar_check:
else
sonar_check:
	@docker run --rm \
                --dns ${DNS_NAMESERVER} \
                -e SONAR_HOST_URL="${SONAR_HOST_URL}" \
                -e SONAR_TOKEN="${SONAR_TOKEN}" \
                -v ".:/usr/src" \
                sonarsource/sonar-scanner-cli
endif

