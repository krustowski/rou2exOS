BITS 64

global int7f_isr
extern syscall_handler

section .text
int7f_isr:
    ; Test the invocation: print '!' with blue fg
    ;mov byte [0xb8000], '!'
    ;mov byte [0xb8001], 0x1F

    ;push rax
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11

    call syscall_handler

    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    ;pop rax

    iretq

