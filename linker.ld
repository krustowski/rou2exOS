ENTRY(_start)

KERNEL_VMA  = 0xffffffff80000000;
KERNEL_LMA  = 0x00200000;           /* physical 2 MiB */

STACK_SIZE  = 0x100000;             /* 1 MiB */
HEAP_SIZE   = 0x200000;             /* 2 MiB */

SECTIONS
{
    /* -------------------------------------------------- */
    /*  Low memory boot section (identity mapped)        */
    /* -------------------------------------------------- */

    . = 0x00100000;

    __kernel_phys_start = .;

    .multiboot2_header ALIGN(8) :
    {
        KEEP(iso/boot/mb2.o(.multiboot2_header))
    }

    .boot ALIGN(4K) :
    {
        KEEP(iso/boot/boot.o(.text))
        KEEP(iso/boot/boot.o(.rodata))
        KEEP(iso/boot/boot.o(.data))
        KEEP(iso/boot/boot.o(.bss))
    }

    /* -------------------------------------------------- */
    /*  Switch to higher-half virtual addresses          */
    /* -------------------------------------------------- */

    . = KERNEL_VMA;

    __kernel_start = .;

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VMA + KERNEL_LMA)
    {
        *(.text*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VMA + KERNEL_LMA)
    {
        *(.rodata*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VMA + KERNEL_LMA)
    {
        *(.data*)
    }

    .bss ALIGN(4K) (NOLOAD) : AT(ADDR(.bss) - KERNEL_VMA + KERNEL_LMA)
    {
        *(COMMON)
        *(.bss*)
    }

    /* -------------------------------------------------- */
    /*  Kernel heap                                       */
    /* -------------------------------------------------- */

    .heap ALIGN(4K) (NOLOAD) :
    {
        __heap_start = .;
        . += HEAP_SIZE;
        __heap_end = .;
    }

    /* -------------------------------------------------- */
    /*  Kernel stack                                      */
    /* -------------------------------------------------- */

    .stack ALIGN(16) (NOLOAD) :
    {
        __stack_bottom = .;
        . += STACK_SIZE;
        __stack_top = .;
    }

    __kernel_end = .;

    __kernel_phys_end = __kernel_end - KERNEL_VMA + KERNEL_LMA;
}

