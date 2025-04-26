build:
	@cargo build --target x86_64-r2.json
	@cargo bootimage

run:
	@qemu-system-x86_64 -drive format=raw,file=target/x86_64-r2/debug/bootimage-kernel.bin
