#[repr(C, packed)]
#[derive(Clone)]
pub struct BootSector {
    pub jmp: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    // Optional extensions ignored
}

#[repr(C, packed)]
pub struct Entry {
    pub name: [u8; 8],         // "FILE    "
    pub ext: [u8; 3],          // "TXT"
    pub attr: u8,
    pub reserved: u8,
    pub create_time_tenths: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub last_access_date: u16,
    pub high_cluster: u16,     // ignored in FAT16
    pub write_time: u16,
    pub write_date: u16,
    pub start_cluster: u16,
    pub file_size: u32,
}

#[repr(C, packed)]
pub struct DirEntry {
    pub name: [u8; 8],         // "FILE    "
    pub ext: [u8; 3],          // "TXT"
    pub typ: u8,
    pub mode: u8,
    pub owner_id: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub last_access_date: u16,
    pub write_time: u16,
    pub write_date: u16,
    pub start_sector: u16,
    pub file_size: u32,
}

