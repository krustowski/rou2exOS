#[repr(C, packed)]
#[cfg(feature = "kernel_text")]
struct Multiboot2HeaderText {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,

    // End tag
    tag_end_type: u16,  // = 0
    tag_end_flags: u16, // = 0
    tag_end_size: u32,  // = 8
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".multiboot2_header")]
#[used]
#[cfg(feature = "kernel_text")]
pub static MULTIBOOT2_HEADER_TEXT: Multiboot2HeaderText = {
    const MAGIC: u32 = 0xE85250D6;
    const ARCH: u32 = 0;
    const HEADER_LEN: u32 = core::mem::size_of::<Multiboot2HeaderText>() as u32;
    const CHECKSUM: u32 = 0u32.wrapping_sub(MAGIC + ARCH + HEADER_LEN);

    Multiboot2HeaderText {
        magic: MAGIC,
        architecture: ARCH,
        header_length: HEADER_LEN,
        checksum: CHECKSUM,

        tag_end_type: 0,
        tag_end_flags: 0,
        tag_end_size: 8,
    }
};

//
//  Graphics kernel with a framebuffer
//

#[repr(C, packed)]
#[cfg(feature = "kernel_graphics")]
struct Multiboot2HeaderGraphics {
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
#[cfg(feature = "kernel_graphics")]
pub static MULTIBOOT2_HEADER_GRAPHICS: Multiboot2HeaderGraphics = {
    const MAGIC: u32 = 0xE85250D6;
    const ARCH: u32 = 0;
    const HEADER_LEN: u32 = core::mem::size_of::<Multiboot2HeaderGraphics>() as u32;
    const CHECKSUM: u32 = 0u32.wrapping_sub(MAGIC + ARCH + HEADER_LEN);

    Multiboot2HeaderGraphics {
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

