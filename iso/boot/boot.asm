;
; boot.asm
; NASM syntax - stage2 stub to set GDT, IDT and memory paging before jumping into 64bit mode
;

BITS 32

section .bss

align 4096
;global dma
;dma:
;	resb 4096

p4_table:           
	resb 4096
p3_table:           
	resb 4096
p2_table:           
	resb 4096

p3_fb_table:           
	resb 4096

align 16
ist1_stack:
    resb 4096
ist1_stack_top:

align 16
tss64:
    resb 104
    
align 4
multiboot_magic:
    resq 1
multiboot_ptr:
    resq 1

align 16
stack_bottom:
    resb 64
stack_top:

;
;  Text Section + Absolute Kernel Entry Point
;

section .text
align 4

extern kernel_main
global start

global multiboot_magic
global multiboot_ptr

global tss64
global dma

global p4_table
global p3_table
global p2_table

global gdt_start
global gdt_end
global gdt_descriptor
global gdt_tss_descriptor
global idt_ptr

global debug_flag
debug_flag:
    db 0    ; 1 = enabled

start:
    mov [multiboot_magic], eax
    mov [multiboot_ptr], ebx

    mov eax, p4_table
    mov cr3, eax

    cli

    call set_up_page_tables

    mov dword [0xb8000 + 80], 0x2f4b2f4f

    ;call load_tss
    call load_gdt
    call load_idt

    mov ax, 0x10      
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    call enable_paging

    ; Load 64-bit code segment and jump
    jmp 0x08:long_mode_entry

    jmp $

load_gdt:
    lgdt [gdt_descriptor]
    ret

load_tss:
    ; Set IST1 stack pointer
    mov eax, ist1_stack_top
    mov [gdt_tss_descriptor + 36], eax

    mov ax, 0x28
    ltr ax
    ret

load_idt:
    lidt [idt_ptr]
    ret

;
;  Global Descriptor Table + Task State Segment
;

section .rodata
align 8

gdt_start:
    ; Null descriptor
    dq 0x0000000000000000

    ; Kernel code segment (offset 0x08)
    dq 0x00AF9A000000FFFF

    ; Kernel data segment (offset 0x10)
    dq 0x00AF92000000FFFF

    ; User code segment (offset 0x18)
    dq 0x00affa000000ffff

    ; User data segment (offset 0x20)
    dq 0x00aff2000000ffff

gdt_tss_descriptor:
    dw 0x0067                 ; limit
    dw 0                      ; base low 16 (will patch)
    db 0                      ; base mid 8 (will patch)
    db 0x89                   ; access byte
    db 0                      ; flags + limit high nibble
    db 0                      ; base high 8
    dd 0                      ; base upper 32 bits
    dd 0                      ; reserved

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dq gdt_start

;
;  Interrupt Descriptor Table
;

; Define a 256-entry dummy IDT (null handlers)
idt_ptr:
    dw idt_end - idt_start - 1  
    dq idt_start            

extern page_fault_handler
idt_start:
    ; Page fault handler (vector 0x0E)
    dq page_fault_handler    
    dw 0x08                   ; Code segment (kernel code segment)
    db 0                    
    db 0x8E                   ; Type: interrupt gate (present, DPL = 0)
    dw 0                     
    dd 0
    dq 0                      

    times 256 dq 0            
idt_end:

;
;  Page Tables Zeroing & Mapping
;

section .text

zero_table:
    mov ecx, 512          
    xor eax, eax        

.zero_loop:
    mov [edi], eax
    add edi, 8
    loop .zero_loop

    ret

set_up_page_tables:
    lea edi, [p4_table]
    call zero_table

    lea edi, [p3_table]
    call zero_table

    lea edi, [p2_table]
    call zero_table

    ; Map P4[0] → P3
    mov eax, p3_table
    or eax, 0b111
    mov [p4_table + 0 * 8], eax
    mov dword [p4_table + 0 * 8 + 4], 0

    mov eax, p3_fb_table
    or eax, 0b111
    mov [p4_table + 1 * 8], eax
    mov dword [p4_table + 1 * 8 + 4], 0

    ; Map P3[0] → P2
    mov eax, p2_table
    or eax, 0b111
    mov [p3_table + 0 * 8], eax
    mov dword [p3_table + 0 * 8 + 4], 0

    ; Identity map 1 GiB (512 runs) using huge pages

    xor ecx, ecx
.map_2mib:
    mov eax, 0x200000
    mul ecx
    or eax, 0b10000011        
    mov [p2_table + ecx * 8], eax
    mov dword [p2_table + ecx * 8 + 4], 0

    inc ecx
    cmp ecx, 512
    jne .map_2mib

    mov ecx, 1
.map_1gib:
    mov eax, 0x40000000
    mul ecx
    or eax, 0b10000011        
    mov [p3_table + ecx * 8], eax
    mov dword [p3_table + ecx * 8 + 4], 0

    inc ecx
    cmp ecx, 3
    jne .map_1gib

    ; Allow CPL=3 access at 0x600_000--0x800_000

    mov eax, 0x600000
    or eax, 0b11100111
    mov [p2_table + 3 * 8], eax
    mov dword [p2_table + 3 * 8 + 4], 0 

    mov eax, 0x800000
    or eax, 0b11100111
    mov [p2_table + 4 * 8], eax
    mov dword [p2_table + 4 * 8 + 4], 0 

    ret

enable_paging:
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ;mov eax, p4_table
    ;mov cr3, eax

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

;
;  Long Mode Entry Point
;

BITS 64

section .text

long_mode_entry:
    ; TLB flush
    mov rax, cr3
    mov cr3, rax

    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Clear the stack
    mov rsp, 0x190000

    mov rsi, [multiboot_ptr]
    mov rdi, [multiboot_magic]

    call kernel_main

    hlt
    jmp $

