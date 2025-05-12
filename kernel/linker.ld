ENTRY(_start)

SECTIONS {
    . = 0x100000; /* load address of kernel in memory (1MB) */

    .multiboot2_header : ALIGN(4K) {
        KEEP(*(.multiboot2_header))
    }

    .text : {
        /**(.text._start)*/
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(.bss*)
        *(COMMON)
    }

    /*.tss : AT(0x1000) {
        *(.tss)    /* TSS will be placed at the specified address (0x1000 in this example) */
    /*}*/

    .gdt : {
        *(.gdt)    /* GDT descriptor data */
    }

    .idt : {
        *(.idt)    /* IDT descriptor data */
    }

    /* Stack */
    . = ALIGN(16);
    stack_bottom = .;
    . = . + 16K; /* 16 KiB */
    stack_top = .;

    /* Define a symbol for the start of the heap */
    . = ALIGN(16);
    __heap_start = .;
    . = . + 64K; /* Reserve 64 KiB for heap */
    __heap_end = .;

    . = ALIGN(4K);
    pml4_table = .;
    . = . + 4K;
    pdpt_table = .;
    . = . + 4K;
    pd_table = .;
    . = . + 4K;
    pt_table = .;
    . = . + 4K;
}

