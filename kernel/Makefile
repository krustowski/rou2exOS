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
	@cargo rustc --release -Z build-std=core,compiler_builtins --target x86_64-r2.json -- --emit=obj

nasm:
	@nasm -f elf64 -o iso/boot/boot.o iso/boot/boot.asm

link:
	@ld.lld --verbose -T linker.ld -n --gc-sections -o iso/boot/kernel.elf target/x86_64-r2/release/deps/kernel-*.o iso/boot/boot.o

build_iso:
	@grub2-mkrescue -o r2.iso iso/ \
		--modules="multiboot2 vbe video video_bochs video_cirrus gfxterm all_video"

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

run_iso_debug: 
	@qemu-system-x86_64 \
		-boot d \
		-m 2G \
		-vga cirrus \
		-cdrom r2.iso \
		-d int,cpu_reset \
		-no-reboot \
		-no-shutdown \
		-serial stdio

#
#  HELPERS
#

clean:
	@cargo clean

clippy:
	@cargo clippy --release --target x86_64-r2.json --no-default-features -- -D warnings

