global int80_stub
extern syscall_handler

section .text
int80_stub:
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

