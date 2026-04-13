BITS 32

section .multiboot2_header
align 8
multiboot2_header:
    dd 0xE85250D6     	; magic
    dd 0              	; arch
    dd 24  		; length
    dd -(0xE85250D6 + 0 + 24)
    ; end tag
    dw 0
    dw 0
    dd 8

