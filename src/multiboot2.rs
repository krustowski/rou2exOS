#[repr(C, packed)]
struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,

    // Framebuffer tag
    tag_fb_type: u16,   // = 5
    tag_fb_flags: u16,  // = 0
    tag_fb_size: u32,   // = 20
    fb_width: u32,      // e.g., 1024
    fb_height: u32,     // e.g., 768
    fb_depth: u32,      // e.g., 32
    fb_pad: u32,

    // End tag
    tag_end_type: u16,  // = 0
    tag_end_flags: u16, // = 0
    tag_end_size: u32,  // = 8
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".multiboot2_header")]
#[used]
pub static MULTIBOOT2_HEADER: Multiboot2Header = {
    const MAGIC: u32 = 0xE85250D6;
    const ARCH: u32 = 0;
    const HEADER_LEN: u32 = core::mem::size_of::<Multiboot2Header>() as u32;
    const CHECKSUM: u32 = 0u32.wrapping_sub(MAGIC + ARCH + HEADER_LEN);

    Multiboot2Header {
        magic: MAGIC,
        architecture: ARCH,
        header_length: HEADER_LEN,
        checksum: CHECKSUM,

        tag_fb_type: 5,
        tag_fb_flags: 0,
        tag_fb_size: 24,
        fb_width: 1024,
        fb_height: 768,
        fb_depth: 32,
        fb_pad: 0,

        tag_end_type: 0,
        tag_end_flags: 0,
        tag_end_size: 8,
    }
};

