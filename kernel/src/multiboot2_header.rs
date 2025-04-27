#[unsafe(link_section = ".text.multiboot2")]
#[unsafe(no_mangle)]
pub static MULTIBOOT2_HEADER: [u32; 5] = [
    0xE85250D6, // multiboot2 magic
    2,          // architecture (0 = i386)
    24,         // total length (in bytes)
    (0xFFFFFFFFu32 - (0xE85250D6u32 + 0 + (8 * 4)) + 1) & 0xFFFFFFFF, // Checksum
    0x100000,
];

