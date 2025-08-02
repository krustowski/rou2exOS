BITS 64

extern tss64
extern kernel_return
;extern syscall_handler_80h
global syscall_80h
;syscall_80h:
;    cli                     
;
;    mov rsp, 0x80000
;    jmp kernel_return
