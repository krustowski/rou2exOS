use crate::fs::block::BlockDevice;

pub struct MemDisk {
    pub data: &'static mut [u8], // Must be sector-aligned
}

impl MemDisk {
    pub fn new(data: &'static mut [u8]) -> Self {
        Self { data }
    }

    fn sector_offset(&self, lba: u64) -> usize {
        (lba as usize) * 512
    }
}

impl BlockDevice for MemDisk {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) {
        let offset = self.sector_offset(lba);
        let slice = &self.data[offset..offset + 512];
        buffer.copy_from_slice(slice);
    }

    fn write_sector(&self, _lba: u64, _buffer: &[u8; 512]) {
        //let offset = self.sector_offset(lba);
        //let slice = &self.data[offset..offset + 512];
        //slice.copy_from_slice(buffer);
    }
}
