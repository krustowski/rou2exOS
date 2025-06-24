; iso/boot/boot.asm
; NASM syntax - Multiboot2 compliant 64-bit kernel loader for GRUB

BITS 32

section .bss
align 4096

pml4_table:    
	resq 512
pdpt_table:    
	resq 512
pd_table:      
	resq 512      
pt_table:      
	resq 512      

p4_table:           
	resb 4096
p3_table:           
	resb 4096
p2_table:           
	resb 4096
p1_page_tables:     
	resb 4096
p1_page_tables_2:   
	resb 4096

p3_fb_table:
    	resq 512
p2_fb_table:
    	resq 512
p1_fb_table:
    	resq 512
p1_fb_table_2:      
	resb 4096

p1_low_table:
    resq 512
p1_extra_table:
    resb 4096
p1_table_page_tables:
    resb 4096

align 16
ist1_stack:
    resb 4096
ist1_stack_top:

align 16
tss:
    ; TSS layout: 104 bytes (enough for x86_64 TSS)
    resb 104
tss_end:

align 4
global multiboot_ptr
multiboot_ptr:
    resq 1

align 16
stack_bottom:
    resb 64
stack_top:

;
;
;

section .text
align 4

extern kernel_main

global p4_table
global p3_table
global p2_table

global p3_fb_table
global p2_fb_table
global p1_fb_table
global p1_fb_table_2

global _start
global debug_flag
debug_flag:
    db 0    ; 1 = enabled

_start:
    mov [multiboot_ptr], ebx

    mov eax, p4_table
    mov cr3, eax

    cli

    call set_up_page_tables

    mov dword [0xb8000 + 80], 0x2f4b2f4f

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
    mov [tss + 36], eax

    mov ax, 0x28
    ltr ax
    ret

load_idt:
    lidt [idt_ptr]
    ret

;
;
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

gdt_tss_low: 
    dq 0
gdt__tss_high:
    dq 0

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dq gdt_start

; Define a 256-entry dummy IDT (null handlers)
idt_ptr:
    dw idt_end - idt_start - 1   ; Limit (size - 1)
    dq idt_start                 ; Base

idt_start:
    ; Page fault handler (vector 0x0E)
    dq page_fault_handler     ; Address of the handler
    dw 0x08                   ; Code segment (kernel code segment)
    db 0                      ; Reserved
    db 0x8E                   ; Type: interrupt gate (present, DPL = 0)
    dw 0                      ; Reserved
    dd 0
    dq 0                      ; Reserved (Upper part of handler address)

    times 256 dq 0            ; 256 null descriptors (each 16 bytes in 64-bit)
idt_end:

; Page Fault Handler (INT 0x0E)
page_fault_handler:
    pusha                      

    mov eax, [esp + 8]         ; Error code
    mov ebx, [esp + 12]        ; Faulting address (CR2)

    popa                       
    iret                       

;
;
;

section .text

%define FB_PHYS     0xFD000000
%define FB_VIRT     0xC0000000
%define PAGE_COUNT  (0x400000 / 0x1000)  ; 4 MiB / 4 KiB = 1024 pages
%define PAGE_FLAGS  0b11

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

    lea edi, [p3_fb_table]
    call zero_table

    lea edi, [p2_table]
    call zero_table

    lea edi, [p2_fb_table]
    call zero_table

    lea edi, [p1_fb_table]
    call zero_table

    lea edi, [p1_page_tables]
    call zero_table

    ; Map P4[0] → P3
    mov eax, p3_table
    or eax, PAGE_FLAGS
    mov [p4_table + 0 * 8], eax
    mov dword [p4_table + 0 * 8 + 4], 0

    ; Map P3[0] → P2
    mov eax, p2_table
    or eax, PAGE_FLAGS
    mov [p3_table + 0 * 8], eax
    mov dword [p3_table + 0 * 8 + 4], 0

    ; Identity map 0–1 GiB with 2 MiB pages
    xor ecx, ecx
.map_1gib:
    mov eax, 0x200000
    mul ecx
    or eax, 0b10000011        
    mov [p2_table + ecx * 8], eax
    mov dword [p2_table + ecx * 8 + 4], 0

    inc ecx
    cmp ecx, 512
    jne .map_1gib

    ;-----------------------------------------
    ; Identity-map your own page table memory
    ; Assume p1_page_tables = 0x134000

    mov eax, p1_page_tables
    or eax, PAGE_FLAGS
    mov [p2_table + 1 * 8], eax   ; Place under p2[1]
    mov dword [p2_table + 1 * 8 + 4], 0

    mov eax, p1_page_tables_2
    or eax, PAGE_FLAGS
    mov [p2_table + 2 * 8], eax   ; Place under p2[1]
    mov dword [p2_table + 2 * 8 + 4], 0

    xor ecx, 0
.map_self:
    mov eax, 0x131000        ; first page table
    add eax, ecx
    or eax, PAGE_FLAGS
    mov edi, p1_page_tables
    mov ebx, ecx
    shr ebx, 12
    shl ebx, 3
    add edi, ebx

    mov [edi], eax
    mov dword [edi + 4], 0

    add ecx, 0x1000
    cmp ecx, 0x40000
    jb .map_self

.map_self_2:
    mov eax, 0x13e000        
    add eax, ecx
    or eax, PAGE_FLAGS
    mov edi, p1_page_tables_2
    mov ebx, ecx
    shr ebx, 12
    shl ebx, 3
    add edi, ebx

    mov [edi], eax
    mov dword [edi + 4], 0

    add ecx, 0x1000
    cmp ecx, 0x40000
    jb .map_self_2

    ; Framebuffer

    mov eax, p3_fb_table
    or eax, PAGE_FLAGS
    mov [p4_table + 511 * 8], eax
    mov dword [p4_table + 511 * 8 + 4], 0

    mov eax, p2_fb_table
    or eax, PAGE_FLAGS
    mov [p3_fb_table + 0 * 8], eax
    mov dword [p3_fb_table + 0 * 8 + 4], 0

    mov eax, p1_fb_table
    or eax, PAGE_FLAGS
    mov [p2_fb_table + 0 * 8], eax
    mov dword [p2_fb_table + 0 * 8 + 4], 0

    xor ecx, ecx
.map_fb:
    mov eax, FB_PHYS
    add eax, ecx
    or eax, PAGE_FLAGS

    mov edi, p1_fb_table
    mov ebx, ecx
    shr ebx, 12 
    shl ebx, 3 
    add edi, ebx

    mov [edi], eax 
    mov dword [edi + 4], 0

    ;mov [p1_fb_table + ecx * 8], eax
    ;mov dword [p1_fb_table + ecx * 8 + 4], 0

    add ecx, 0x1000
    cmp ecx, PAGE_COUNT
    jb .map_fb

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
;
;

BITS 64

section .text

enable_paging_64:
    mov rax, pml4_table
    mov cr3, rax

    ; CR4: enable PAE
    mov rax, cr4
    or rax, 1 << 5        ; PAE
    mov cr4, rax

    ; EFER: enable long mode
    mov ecx, 0xC0000080   ; EFER MSR
    rdmsr
    or eax, 1 << 8        ; LME
    wrmsr

    ; CR0: enable paging
    mov rax, cr0
    or rax, 1 << 31       ; PG
    mov cr0, rax

    ret


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

    ; Clear the stack (assume there'S 1MB+ memory)
    mov rsp, 0x80000
    call kernel_main

    hlt
    jmp $

