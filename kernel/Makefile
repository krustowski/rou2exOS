init:
	@ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
	@. "${HOME}/.cargo/env" && \
		rustup install nightly && \
		rustup default nightly && \
		rustup target add x86_64-unknown-none && \
		rustup component add rust-src llvm-tools-preview && \
		cargo install bootimage

build:
	@. "${HOME}/.cargo/env" && \
		cargo build --target x86_64-r2.json && \
		cargo bootimage

run:
	@qemu-system-x86_64 -drive format=raw,file=target/x86_64-r2/debug/bootimage-x86_64-r2.bin
