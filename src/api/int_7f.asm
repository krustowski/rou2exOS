global int7f_isr
extern syscall_handler

section .text
int7f_isr:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    call syscall_handler
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    iretq

