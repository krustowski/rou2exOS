const PAGE_SIZE: u64 = 4096;
const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITE: u64 = 1 << 1;
const PAGE_FLAGS: u64 = PAGE_PRESENT | PAGE_WRITE;

pub fn map_framebuffer(
    fb_phys_addr: u64,
    fb_virt_base: u64,
    fb_size: u64,
    p4_table: &mut [u64; 512],
    p3_fb_table: &mut [u64; 512],
    p2_fb_tables: &mut [&mut [u64; 512]],  
    p1_fb_tables: &mut [&mut [u64; 512]],
) {
    let page_count = fb_size.div_ceil(PAGE_SIZE);

    for i in 0..page_count {
        let virt = fb_virt_base + i * PAGE_SIZE;
        let phys = fb_phys_addr + i * PAGE_SIZE;

        let p4i = ((virt >> 39) & 0x1FF) as usize;
        let p3i = ((virt >> 30) & 0x1FF) as usize;
        let p2i = ((virt >> 21) & 0x1FF) as usize;
        let p1i = ((virt >> 12) & 0x1FF) as usize;

        // Link P4 → P3
        if p4_table[p4i] & PAGE_PRESENT == 0 {
            p4_table[p4i] = (p3_fb_table.as_ptr() as u64) | PAGE_FLAGS;
        }

        // Link P3 → P2
        if p3_fb_table[p3i] & PAGE_PRESENT == 0 {
            let p2_table = &mut p2_fb_tables[p3i];
            p3_fb_table[p3i] = (p2_table.as_ptr() as u64) | PAGE_FLAGS;
        }

        // Link P2 → P1
        let p2_table = &mut p2_fb_tables[p3i];
        if p2_table[p2i] & PAGE_PRESENT == 0 {
            let p1_index = (p3i << 9) | p2i;
            let p1_table = &p1_fb_tables[p1_index];
            p2_table[p2i] = (p1_table.as_ptr() as u64) | PAGE_FLAGS;
        }

        // Map the physical framebuffer page into P1
        let p1_index = (p3i << 9) | p2i;
        let p1_table = &mut p1_fb_tables[p1_index];
        p1_table[p1i] = phys | PAGE_FLAGS;
    }
}

/*pub fn print_result(fb: &super::multiboot_parser::FramebufferTag) -> super::result::InitResult {
    use crate::video;

    video::mode::init_video(fb);

    if video::mode::get_video_mode().is_some() {
        return super::result::InitResult::Passed;
    }

    super::result::InitResult::Failed
}
*/