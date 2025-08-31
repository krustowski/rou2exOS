ENTRY(start)

SECTIONS {
    . = 0x100000; /* 1 MB offset */

    .multiboot2_header : ALIGN(4K) {
        KEEP(*(.multiboot2_header))
    }

    __kernel_start = .;

    .text : {
	*(.text.entry)
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
	*(.dma)
    }

    .bss : {
        *(.bss*)
        *(COMMON)
    }

    .gdt : {
        *(.gdt)  
    }

    .idt : {
        *(.idt)   
    }

    /* Stack */
    . = ALIGN(16);
    stack_bottom = .;
    . = . + 64K; 
    stack_top = .;

    /* Kernel Heap */
    . = ALIGN(16);
    __heap_start = .;
    . = . + 128K; 
    __heap_end = .;

    . = ALIGN(4K);
    pml4_table = .;    . = . + 4K;
    pdpt_table = .;    . = . + 4K;
    pd_table   = .;    . = . + 4K;
    pt_table   = .;    . = . + 4K;

    p4_table     = .;  . = . + 4K;
    p3_fb_table  = .;  . = . + 4K;
    p2_fb_table  = .;  . = . + 4K;
    p1_fb_table  = .;  . = . + 4K;

    p1_fb_table_0  = .;  . = . + 4K;
    p1_fb_table_1  = .;  . = . + 4K;
    p1_fb_table_2  = .;  . = . + 4K;
    p1_fb_table_3  = .;  . = . + 4K;

    __kernel_end = .;

    . = 0x650000;
    .user_task : {
	 *(.user_task*)
    }

    . = 0x80000; 
    .dma : ALIGN(512) {
        KEEP(*(.dma))
    }

}

