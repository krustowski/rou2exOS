; iso/boot/boot.asm
; NASM syntax - Multiboot2 compliant 64-bit kernel loader for GRUB

BITS 32

;section .multiboot2_header
;align 8

;MB2_MAGIC        equ 0xe85250d6
;MB2_ARCH         equ 0x0
;MB2_HEADER_LEN   equ header_end - header_start
;MB2_CHECKSUM     equ -(MB2_MAGIC + MB2_ARCH + MB2_HEADER_LEN)

;header_start:
;    dd MB2_MAGIC
;    dd MB2_ARCH
;    dd MB2_HEADER_LEN
;    dd MB2_CHECKSUM

   ; Framebuffer tag
    ;dw 5 	; type (framebuffer)
    ;dw 0        ; flags
    ;dd 24	; size
    ;dd 1024	; width
    ;dd 768	; height
    ;dd 32	; depth
    ;dd 0

    ; End tag (type = 0, size = 8)
;    dw 0
;    dw 0
;    dd 8
;header_end:

section .bss
align 4096
p4_table:
    resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
;stack_bottom:
;    resb 64
;stack_top:

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

;align 16
;tss:
    ; IST pointers (7 entries), each 8 bytes
    ; We'll only use IST1 for example
;    resq 1                  ; RSP0 (not used but must exist)
;    resq 1                  ; RSP1
;    resq 1                  ; RSP2
;    resq 1                  ; IST1
;    resq 6                  ; IST2–IST7
;    resq 1                  ; Reserved
;    resw 0                   ; IO Map base (disable I/O map)
;    resw 0
;tss_end:


section .text
align 4

extern rust_main

extern pml4_table
extern pdpt_table
extern pd_table
extern pt_table

global _start

_start:
    mov [multiboot_ptr], ebx

    cli

    ;call setup_paging
    call set_up_page_tables
    call enable_paging

    mov dword [0xb8000 + 80], 0x2f4b2f4f
    ;hlt

    call load_gdt
    call load_idt

    ; Set protected mode segments
    mov ax, 0x10       ; data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ;call enable_long_mode

    ; Load 64-bit code segment and jump
    jmp 0x08:long_mode_entry
    ;jmp $

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

; === Set up page tables ===
setup_paging:
    ; Clear tables (optional but recommended)
    ; This can be done in Rust too later

    ; --- PML4[511] → PDP @ 0x2000 ---
    mov eax, pdpt_table
    or eax, 0x03        ; Present + Writable
    mov [0x1000 + 8 * 511], eax  ; PML4 entry pointing to PDPT

    ; --- PDP[0] → PD @ 0x3000 ---
    mov eax, pd_table
    or eax, 0x03
    mov [0x2000 + 8 * 0], eax

    ; --- PD[0] → 2 MiB large page at 0x00100000 (kernel) ---
    mov eax, 0x00100000        ; Kernel page starting at 0x00100000
    or eax, 0x83               ; Present + Writable + LargePage
    mov [0x3000 + 8 * 0], eax    ; PD entry pointing to 2MB page

    ; --- (Optional) Identity map 0x00000000 for safe booting ---
    mov eax, 0x00000000        ; Map the first 4KiB
    or eax, 0x03               ; Present + Writable
    mov [0x4000 + 8*0], eax    ; PT entry

    ret

    ; === Enable Long Mode Paging ===
enable_long_mode:
    ; Load PML4 into CR3
    mov eax, pml4_table        ; Make sure this address is correct for your PML4 table
    ;mov eax, 0x1000        ; Make sure this address is correct for your PML4 table
    mov cr3, eax

    ; Enable PAE (Physical Address Extension, necessary for 64-bit)
    mov eax, cr4
    or eax, 1 << 5         ; Set the PAE bit in CR4
    mov cr4, eax

    ; Enable Long Mode (EFER.LME = 1)
    mov ecx, 0xC0000080    ; EFER MSR
    rdmsr                   ; Read MSR
    or eax, 1 << 8          ; Set LME bit to enable long mode
    wrmsr                   ; Write back to EFER MSR

    ; Enable Paging (CR0.PG = 1)
    mov eax, cr0
    or eax, 1 << 31        ; Set paging bit in CR0
    mov cr0, eax

    ret

section .rodata
align 8
gdt_start:
    ; Null descriptor
    dq 0x0000000000000000

    ; Kernel code segment (offset 0x08)
    dq 0x00af9a000000ffff

    ; Kernel data segment (offset 0x10)
    dq 0x00af92000000ffff

    ; User code segment (offset 0x18)
    dq 0x00affa000000ffff

    ; User data segment (offset 0x20)
    dq 0x00aff2000000ffff

    ; TSS descriptor (offset 0x28, needs two entries: 16 bytes total)
    ;dw tss_end - tss - 1               ; [0:15]  Limit 0:15
    ;dw tss               ; [16:31] Base 0:15
    ;db tss >> 16              ; [32:39] Base 16:23
    ;db 0x89                        ; [40:47] Type=0x9 (TSS), Present=1
    ;db 0x00              ; [48:51] Limit 16:19
    ;db tss >> 24              ; [52:59] Base 24:31
    ;dd tss >> 32              ; [64:95] Base 32:63
    ;dd 0x00                         ; [96:127] Reserved
gdt_tss_low: 
    dq 0
gdt__tss_high:
    dq 0

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dq gdt_start


;tss:
;    dq 0x0                    ; Previous TSS Link (set to 0 for no previous task)
;    dq 0x0                    ; ESP0 (main stack address) - Hardcoded address
;    dw 0x10                   ; SS0 (stack segment for privilege level 0)
;    dw 0x0                    ; Unused padding
;    dq ist1_stack_top         ; IST1 (interrupt stack 1) - Hardcoded address
;    dq 0x0                    ; Unused IST2, IST3, etc., (can be set to 0 if not needed)
;    dq 0x0
;    dq 0x0
;    dq 0x0
;    dq 0x0
;    dq 0x0
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;    dq 0x0                    ; Unused fields (reserved)
;tss_end:


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

    times 256 dq 0               ; 256 null descriptors (each 16 bytes in 64-bit)
idt_end:

; Page Fault Handler (INT 0x0E)
page_fault_handler:
    pusha                      ; Save registers

    ; Get the error code and fault address from the stack
    mov eax, [esp + 8]         ; Error code
    mov ebx, [esp + 12]        ; Faulting address (CR2)

    ; Check for specific error codes or handle them
    ; Error code: [Reserved|ID|Reserved|User|Fault|Protection]
    ; More logic to handle fault types goes here

    ; For example, if the page was not present:
    ; - You might want to load the page from swap or handle the fault

    ; In this example, we'll just print a message (you can print via serial port or VGA)
    ; Here you can put your error handling code or panic, depending on your requirements.

    hlt                        ; Halt the system (in case of an unhandled fault)

    popa                       ; Restore registers
    iret                       ; Return from interrupt

;
;
;

set_up_page_tables:
    ; map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11 ; present + writable
    mov [p4_table], eax

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0b11 ; present + writable
    mov [p3_table], eax

    ; map each P2 entry to a huge 2MiB page
    mov ecx, 0         ; counter variable

.map_p2_table:
    ; map ecx-th P2 entry to a huge page that starts at address 2MiB*ecx
    mov eax, 0x200000  ; 2MiB
    mul ecx            ; start address of ecx-th page
    or eax, 0b10000011 ; present + writable + huge
    mov [p2_table + ecx * 8], eax ; map ecx-th entry

    inc ecx            ; increase counter
    cmp ecx, 512       ; if counter == 512, the whole P2 table is mapped
    jne .map_p2_table  ; else map the next entry

    ret

enable_paging:
    ; load P4 to cr3 register (cpu uses this to access the P4 table)
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4 (Physical Address Extension)
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit in the EFER MSR (model specific register)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in the cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

BITS 64

long_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax

    ;call load_tss
    ;mov ax, 0x28
    ;ltr ax

    ; Clear the stack (assume we got 1MB+ memory)
    mov rsp, 0x80000
    call rust_main

    hlt
    jmp $

