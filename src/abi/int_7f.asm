BITS 64

global int7f_isr
extern syscall_handler

section .text
int7f_isr:
    ; Test the invocation: print '!'
    ;mov byte [0xb8000], '!'
    ;mov byte [0xb8001], 0x1F

    ;push rax
    ;push rdi
    ;push rsi
    ;push rdx
    ;push rcx
    ;push r8
    ;push r9

    call syscall_handler

    ;pop r9
    ;pop r8
    ;pop rcx
    ;pop rdx
    ;pop rsi
    ;pop rdi
    ;pop rax

    iretq

