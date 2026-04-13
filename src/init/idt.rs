use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use crate::abi::idt::{install_isrs, load_idt};

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();
static mut GDT: Option<GlobalDescriptorTable> = None;

pub const KERNEL_STACK_TOP: u64 = 0xFFFF_8000_0019_0000;

static mut P4_TABLE: [u64; 512] = [0; 512];

const IST_STACK_SIZE: usize = 4096;

#[repr(align(16))]
struct AlignedStack([u8; IST_STACK_SIZE]);

static mut IST0_STACK: AlignedStack = AlignedStack([0; IST_STACK_SIZE]);
static mut IST1_STACK: AlignedStack = AlignedStack([0; IST_STACK_SIZE]);

fn init_gdt_tss() {
    unsafe {
        // Allocate GDT in high-half memory (just static mutable is fine in Rust)
        GDT = Some(GlobalDescriptorTable::new());

        let gdt = GDT.as_mut().unwrap();

        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());

        let user_code_selector = gdt.append(Descriptor::user_code_segment());
        let user_data_selector = gdt.append(Descriptor::user_data_segment());

        // TSS entry
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));

        // Load GDT
        gdt.load();

        // Load TSS
        x86_64::instructions::tables::load_tss(tss_selector);
    }
}

fn init_tss_stacks() {
    unsafe {
        TSS.privilege_stack_table[0] = VirtAddr::new(KERNEL_STACK_TOP);

        TSS.interrupt_stack_table[0] =
            VirtAddr::new((&IST0_STACK.0 as *const _ as u64) + IST0_STACK.0.len() as u64);
        TSS.interrupt_stack_table[1] =
            VirtAddr::new((&IST1_STACK.0 as *const _ as u64) + IST1_STACK.0.len() as u64);

        TSS.iomap_base = core::mem::size_of::<TaskStateSegment>() as u16;
    }
}

//
//
//
//
//

use crate::video::sysprint::Result;

pub fn idt_cpu_tables() -> Result {
    install_isrs();

    load_idt();

    init_gdt_tss();

    Result::Passed
}

//
//
//

fn make_tss_descriptor(base: u64, limit: u32) -> [u8; 16] {
    let mut desc = [0u8; 16];

    // Limit low 16 bits
    desc[0] = (limit & 0xFF) as u8;
    desc[1] = ((limit >> 8) & 0xFF) as u8;

    // Base low 16 bits
    desc[2] = (base & 0xFF) as u8;
    desc[3] = ((base >> 8) & 0xFF) as u8;

    // Base middle 8 bits
    desc[4] = ((base >> 16) & 0xFF) as u8;

    // Access byte: present, privilege level 0, system segment, type 0x9 (available 64-bit TSS)
    desc[5] = 0b10001001; // P=1, DPL=00, S=0, Type=1001b = 0x9

    // Flags: limit high 4 bits + granularity + 64-bit flag + others
    desc[6] = ((limit >> 16) & 0xF) as u8;

    // Base high 8 bits
    desc[7] = ((base >> 24) & 0xFF) as u8;

    // Base upper 32 bits (for 64-bit base address)
    desc[8] = ((base >> 32) & 0xFF) as u8;
    desc[9] = ((base >> 40) & 0xFF) as u8;
    desc[10] = ((base >> 48) & 0xFF) as u8;
    desc[11] = ((base >> 56) & 0xFF) as u8;

    desc[12] = 0;
    desc[13] = 0;
    desc[14] = 0;
    desc[15] = 0;

    desc
}
