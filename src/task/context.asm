BITS 64

section .text
global context_switch
global context_switch_kern

; rdi = old_regs_ptr, rsi = new_regs_ptr
context_switch:
    test rsi, rsi
    jz .fail
    mov rax, [rsi + 0x78]
    test rax, rax
    jz .fail
    ; ... continue

    ; 1) Save all GPRs into *old
    mov [rdi + 0x38], r8
    mov r8, rdi

    mov [r8 + 0x00], r15
    mov [r8 + 0x08], r14
    mov [r8 + 0x10], r13
    mov [r8 + 0x18], r12
    mov [r8 + 0x20], r11
    mov [r8 + 0x28], r10
    mov [r8 + 0x30], r9
    mov [r8 + 0x40], rdi
    mov [r8 + 0x48], rsi
    mov [r8 + 0x50], rbp
    mov [r8 + 0x58], rdx
    mov [r8 + 0x60], rcx
    mov [r8 + 0x68], rbx
    mov [r8 + 0x70], rax

    ; 2) Load all GPRs from *new
    mov r15, [rsi + 0x00]
    mov r14, [rsi + 0x08]
    mov r13, [rsi + 0x10]
    mov r12, [rsi + 0x18]
    mov r11, [rsi + 0x20]
    mov r10, [rsi + 0x28]
    mov r9 , [rsi + 0x30]
    mov r8 , [rsi + 0x38]
    mov rdi, [rsi + 0x40]
    mov rbp, [rsi + 0x50]    ; if you really meant to load saved RBP
    mov rdx, [rsi + 0x58]
    mov rcx, [rsi + 0x60]
    mov rbx, [rsi + 0x68]
    mov rax, [rsi + 0x70]

    mov r10, [rsi + 0x78]    ; new.rip
    mov r11, [rsi + 0x80]    ; new.cs
    mov r12, [rsi + 0x88]    ; new.rflags
    mov r13, [rsi + 0x90]    ; new.rsp
    mov r14, [rsi + 0x98]    ; new.ss

    mov rsi, [rsi + 0x48]

    ; 4) Build the return frame on the stack for iretq
    push r14                 ; SS
    push r13                 ; RSP
    push r12                 ; RFLAGS
    push r11                 ; CS
    push r10                 ; RIP

    ; 5) Finally, do the far return
    iretq

.fail:
    hlt


context_switch_kern:
    ; Save all general-purpose registers to *old
    mov [rdi + 0x00], r15
    mov [rdi + 0x08], r14
    mov [rdi + 0x10], r13
    mov [rdi + 0x18], r12
    mov [rdi + 0x20], r11
    mov [rdi + 0x28], r10
    mov [rdi + 0x30], r9
    mov [rdi + 0x38], r8
    mov [rdi + 0x40], rdi
    mov [rdi + 0x48], rsi
    mov [rdi + 0x50], rbp
    mov [rdi + 0x58], rdx
    mov [rdi + 0x60], rcx
    mov [rdi + 0x68], rbx
    mov [rdi + 0x70], rax

    ; Save current RIP manually using call/pop
    call .save_rip

.save_rip:
    pop rax
    mov [rdi + 0x78], rax ; RIP

    ; Save current RSP
    mov rax, rsp
    mov [rdi + 0x80], rax

    ; Save RFLAGS
    pushfq
    pop rax
    mov [rdi + 0x88], rax

    ; Load all registers from *new
    mov r15, [rsi + 0x00]
    mov r14, [rsi + 0x08]
    mov r13, [rsi + 0x10]
    mov r12, [rsi + 0x18]
    mov r11, [rsi + 0x20]
    mov r10, [rsi + 0x28]
    mov r9 , [rsi + 0x30]
    mov r8 , [rsi + 0x38]
    mov rdi, [rsi + 0x40]
    mov rsi, [rsi + 0x48]
    ;mov rbp, [rsi + 0x50]
    mov rbp, 0x394000
    mov rdx, [rsi + 0x58]
    mov rcx, [rsi + 0x60]
    mov rbx, [rsi + 0x68]
    mov rax, [rsi + 0x70]

    ; Load RSP
    ;mov rsp, [rsi + 0x80]
    mov rsp, 0x390000

    ; Restore RFLAGS
    mov rax, [rsi + 0x88]
    push rax
    popfq

    ; Jump to new RIP
    jmp [rsi + 0x78]

