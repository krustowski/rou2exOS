ENTRY(_start)

SECTIONS {
    . = 1M; /* load address of kernel in memory (1MB) */

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

    /* Define a symbol for the start of the heap */
    . = ALIGN(8);
    __heap_start = .;
    . = . + 64K; /* Reserve 64 KiB for heap */
    __heap_end = .;
}

