; boot.asm (32-bit real mode)
[bits 32]
[global _start]

_start:
    cli
    mov esp, stack_top
    call kernel_main
.hang:
    hlt
    jmp .hang

section .bss
resb 4096
stack_top:

