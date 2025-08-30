use std::process::Command;

macro_rules! nasm {
    ($inp:literal, $out:literal) => {
        let res = Command::new("nasm")
            .arg("-f")
            .arg("elf64")
            .arg("-o")
            .arg($out)
            .arg($inp)
            .status()
            .unwrap()
            .success();
        assert!(res);
        println!("cargo::rustc-link-arg={}", $out);
    };
}

fn main() {
    nasm!("iso/boot/boot.asm", "iso/boot/boot.o");
    nasm!("src/abi/int_7f.asm", "src/abi/int_7f.o");
    nasm!("src/abi/int_80.asm", "src/abi/int_80.o");
    nasm!("src/task/context.asm", "src/task/context.o");
}