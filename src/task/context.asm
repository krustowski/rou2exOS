BITS 64

section .text
global context_switch

; rdi = &old_rsp
; rsi = &new_rsp
context_switch:
    mov [rdi], rsp
    mov rsp, [rsi]
    ret
