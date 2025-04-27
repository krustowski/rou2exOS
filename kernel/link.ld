/*
 * Custom linker script for the `cargo` build.
 * Makes sure the multiboot2-header is at the beginning of the file.
 */

/** The "start"-symbol from start.S. */
ENTRY(start)

SECTIONS {

    /* 8MiB works in QEMU and two of my personal computers. Anyhow, it is insane
       to rely on static addresses, because each UEFI implementations may rely on
       other addresses being available. Eventually we will have to fix
       the relocation problem.
       */
    . = 8M;

    /* Multiboot2-Header must be 64-bit (8 byte) aligned according to spec. */
    . = ALIGN(8);

    /* this fails because of: https://stackoverflow.com/questions/68475415/ */
    /*.multiboot2_header :
    {

    }*/

    .text . :
    {
        /* temporary workaround https://stackoverflow.com/questions/68475415/ */
        *(.multiboot2_header)
        *(.text .text.*)
    }

    .rodata :
    {
      *(.rodata .rodata.*)
    }

    .data :
    {
      *(.data .data.*)
    }

    .bss :
    {
      *(COMMON)
      *(.bss .bss.*)
    }
}
