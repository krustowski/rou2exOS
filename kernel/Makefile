init:
	@ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@. "${HOME}/.cargo/env" && \
		rustup install nightly && \
		rustup default nightly && \
		rustup target add x86_64-unknown-none && \
		rustup component add rust-src llvm-tools-preview && \
		cargo install bootimage

#@cargo rustc --release --target x86_64-r2.json -- -C relocation-model=static --emit=obj
build:
	@cargo rustc --release -Z build-std=core,compiler_builtins --target x86_64-r2.json -- --emit=obj
	@ld.lld -T linker.ld -n --gc-sections -o iso/boot/kernel.elf target/x86_64-r2/release/deps/kernel-*.o
	@grub2-mkrescue -o r2.iso iso/

run:
	@qemu-system-x86_64 \
		-serial pty \
		-drive format=raw,file=target/x86_64-r2/debug/bootimage-x86_64-r2.bin

run_iso: 
	@qemu-system-x86_64 -cdrom r2.iso

clean:
	@cargo clean

clippy:
	@cargo clippy --release --target x86_64-r2.json --no-default-features -- -D warnings

