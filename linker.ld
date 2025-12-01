ENTRY(_start)

SECTIONS {
    . = 0x100000; /* 1 MB offset */

    .multiboot2_header : ALIGN(4K) {
        KEEP(*(.multiboot2_header))
    }

    .text : {
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
    __stack_bottom = .;
    . = . + 64K; 
    __stack_top = .;

    /* Heap */
    . = ALIGN(16);
    __heap_start = .;
    . = . + 64K; 
    __heap_end = .;

    . = ALIGN(4K);
    p4_table     = .;  . = . + 4K;
    p3_fb_table  = .;  . = . + 4K;

    /* Experimental, TBD */
    . = 0x650000;
    .user_task : {
	 *(.user_task*)
    }

    . = 0x80000; 
    .dma : ALIGN(512) {
        KEEP(*(.dma))
    }
}

