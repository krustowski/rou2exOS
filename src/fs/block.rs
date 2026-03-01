pub trait BlockDevice {
    /// Reads 1 sector (usually 512 bytes) at the given LBA into `buffer`
    fn read_sector(&self, lba: u64, buffer: &mut [u8]);

    /// Writes 1 sector from `buffer` to `lba`
    fn write_sector(&self, lba: u64, buffer: &[u8; 512]);
}
