
#[repr(C)]
#[derive(Copy,Clone)]
pub struct TagHeader {
    pub typ: header::M2TagType,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct MemoryMapTag {
    pub typ: u32,       
    pub size: u32,          
    pub entry_size: u32,
    pub entry_version: u32,
	pub entries: MemoryMapEntry              
}


#[repr(C, packed)]
#[derive(Debug)]
pub struct MemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub typ: u32,   
    pub reserved: u32,  
}

#[derive(Clone,Copy,Default)]
#[repr(C, packed)]
pub struct FramebufferTag {
    pub typ: u32,
    pub size: u32,
    pub addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub fb_type: u8,
    pub reserved: u16,
}

#[repr(C, packed)]
pub struct AcpiRSDPTag {
    pub typ: u32,
    pub size: u32,
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_addr: u32,
}

#[repr(C, packed)]
pub struct AcpiSDTHeader {
    pub signature: [u8; 4], //array
    pub length: u32,
    pub revision: u8, 
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creatpr_revision: u32,
}

#[repr(C, packed)]
pub struct UsableMemory {
	start: u64,
	end: u64,
	count: u8,

}

