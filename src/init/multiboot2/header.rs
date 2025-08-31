pub const MULTIBOOT_HEADER: u32 = 1;

pub const MULTIBOOT_SEARCH: u32 = 32768;

pub const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe85250d6;
pub const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36d76289;

pub const MULTIBOOT_MOD_ALIGN: u32 = 0x00001000;
pub const MULTIBOOT_INFO_ALIGN: u32 = 0x00000008;
pub const MULTIBOOT_TAG_ALIGN: u32 = 8;
pub const MULTIBOOT_HEADER_ALIGN: u8 = 8;

#[derive(Copy,Clone,PartialEq)]

pub enum M2TagType {
    End              	= 0,
    CmdLine          	= 1,
    BootLoaderName 		= 2,
    Module          	= 3,
    BasicMemoryInfo    	= 4,
    BootDev         	= 5,
    Mmap             	= 6,
    VBE              	= 7,
    Framebuffer      	= 8,
    ElfSections     	= 9,
    APM              	= 10,
    EFI32            	= 11,
    EFI64            	= 12,
    SMBIOS           	= 13,
    AcpiOLD         	= 14,
    AcpiNEW         	= 15,
    Network          	= 16,
    EFIMmap         	= 17,
    EFIBs           	= 18,
    EFI32IH         	= 19,
    EFI64IH         	= 20,
    LoadBaseAddr   		= 21,
}

pub enum M2HeaderTag {
    End  = 0,
    InformationRequest = 1,
    Address = 2,
    EntryAddress = 3,
    ConsoleFlags = 4,
    Framebuffer = 5,
    ModuleAlign = 6,
    EFIBS       = 7,
    EntryAddressEFI32 = 8,
    EntryAddressEFI64 = 9,
    Relocatable = 10,
}

pub enum M2Memory {
    Avaible					= 1,
    Reserved               	= 2,
    AcpiReclaimable       	= 3,
    NVS                    	= 4,
    BADRAM                 	= 5,
}

pub const MULTIBOOT_ARCHITECTURE_I386: u8 = 0;
pub const MULTIBOOT_ARCHITECTURE_MIPS32: u8 =  4;
pub const MULTIBOOT_HEADER_TAG_OPTIONAL: u8 = 1;

pub enum MultibootLoadPreference {
    MultibootLoadPreferenceNone = 0,
    MultibootLoadPreferenceLow  = 1,
    MultibootLoadPreferenceHigh = 2,
}

const MULTIBOOT_CONSOLE_FLAGS_CONSOLE_REQUIRED: u8 = 1;
const MULTIBOOT_CONSOLE_FLAGS_EGA_TEXT_SUPPORTED: u8 = 2;
