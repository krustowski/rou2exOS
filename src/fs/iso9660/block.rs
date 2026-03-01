use crate::fs::block::BlockDevice;

pub struct DirectoryRecord {
    length: u8,
    ext_attr_length: u8,
    extent_lba_le: u32,
    extent_lba_be: u32,
    data_length_le: u32,
    data_length_be: u32,
    datetime: [u8; 7],
    flags: u8,
    file_unit_size: u8,
    interleave_gap_size: u8,
    volume_seq_number_le: u16,
    volume_seq_number_be: u16,
    file_identifier_length: u8,
}

/*impl DirectoryRecord {
    pub fn parse(buf: &[u8]) -> Result<Self, FsError> {
        let length = buf[0];
        if length == 0 {
            return Err(FsError::InvalidRecord);
        }

        let extent_lba = u32::from_le_bytes([buf[2], buf[3], buf[4], buf[5]]);

        let size = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);

        let flags = buf[25];
        let name_len = buf[32] as usize;

        let name_bytes = &buf[33..33 + name_len];

        let mut name = heapless::String::<32>::new();
        for &b in name_bytes {
            if b == b';' {
                break; // strip version ;1
            }
            name.push(b as char).ok();
        }

        Ok(Self {
            extent_lba,
            size,
            flags,
            name,
        })
    }
}*/

pub struct Iso9660 {
    pub data: &'static mut [u8],
}

impl BlockDevice for Iso9660 {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) {
        //
    }

    fn write_sector(&self, lba: u64, buffer: &[u8; 512]) {
        //
    }

    /*pub fn mount(mut device: BlockDevice) -> Result<Self, FsError> {
        let mut sector = [0u8; 2048];

        device.read_sector(16, &mut sector);

        if &sector[1..6] != b"CD001" {
            return Err(FsError::InvalidFs);
        }

        if sector[0] != 1 {
            return Err(FsError::NoPrimaryDescriptor);
        }

        let root = DirectoryRecord::parse(&sector[156..])?;

        Ok(Self { device, root })
    }*/
}
