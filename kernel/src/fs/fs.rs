use crate::fs::block::BlockDevice;
use crate::fs::entry::{BootSector, Entry};
use crate::init::config::DEBUG;

pub struct Fs<'a, D: BlockDevice> {
    pub device: &'a D,
    pub boot_sector: BootSector,
    pub fat_start_lba: u64,
    pub root_dir_start_lba: u64,
    pub data_start_lba: u64,
    pub sectors_per_cluster: u8,
}

impl<'a, D: BlockDevice> Fs<'a, D> {
    pub fn new(device: &'a D, vga_index: &mut isize) -> Result<Self, &'static str> {
        let mut sector = [0u8; 512];
        device.read_sector(0, &mut sector, vga_index);

        let mut found_fat = false;

        for i in 0..512 - 5 {
            if &sector[i..i+5] == b"FAT12" {
                if DEBUG {
                    crate::vga::write::string(vga_index, b"Found FAT12", crate::vga::buffer::Color::Green);
                    crate::vga::write::newline(vga_index);
                }

                found_fat = true;
                break;
            }
            /*if let Some(b) = sector.get(i) {
              crate::vga::write::number(vga_index, *b as u64);
              }*/
        }

        if !found_fat {
            return Err("Could not find the FAT12 label, floppy may not be present");
        }

        let boot_sector = unsafe { (*(sector.as_ptr() as *const BootSector)).clone() };

        if DEBUG {
            crate::vga::write::string(vga_index, b"OEM: ", crate::vga::buffer::Color::White);
            for b in &boot_sector.oem {
                crate::vga::write::byte(vga_index, *b, crate::vga::buffer::Color::Green);
            }
            crate::vga::write::newline(vga_index);
        }

        if DEBUG {
            crate::vga::write::string(vga_index, b"Bytes/Sector: ", crate::vga::buffer::Color::White);
            crate::vga::write::number(vga_index, boot_sector.bytes_per_sector as u64);
            crate::vga::write::newline(vga_index);
        }

        let fat_start = boot_sector.reserved_sectors as u64;
        let root_dir_sectors =
            ((boot_sector.root_entry_count as u32 * 32) + 511) / 512;
        let root_dir_start = fat_start + (boot_sector.fat_count as u64 * boot_sector.fat_size_16 as u64);
        let data_start = root_dir_start + root_dir_sectors as u64;

        Ok(Self {
            device,
            boot_sector: boot_sector.clone(),
            fat_start_lba: fat_start,
            root_dir_start_lba: root_dir_start,
            data_start_lba: data_start,
            sectors_per_cluster: boot_sector.sectors_per_cluster,
        })
    }

    fn cluster_to_lba(&self, cluster: u16) -> u64 {
        self.data_start_lba + ((cluster as u64 - 2) * self.sectors_per_cluster as u64)
    }

    //fn read_file(&self, start_cluster: u16, mut callback: impl FnMut(&[u8]), vga_index: &mut isize) {
    fn read_file(&self, start_cluster: u16, vga_index: &mut isize) {
        let mut current_cluster = start_cluster;
        let mut sector_buf = [0u8; 512];

        loop {
            let lba = self.cluster_to_lba(current_cluster);
            self.device.read_sector(lba, &mut sector_buf, vga_index);

            if DEBUG {
                crate::vga::write::string(vga_index, &sector_buf, crate::vga::buffer::Color::Yellow);
            }

            current_cluster = self.read_fat12_entry(current_cluster, vga_index);
            if current_cluster >= 0xFF8 {
                break; // End of chain
            }
        }
    }

    pub fn read_fat12_entry(&self, cluster: u16, vga_index: &mut isize) -> u16 {
        let fat_offset = (cluster as usize * 3) / 2;
        let sector = (fat_offset / 512) as u64;
        let offset_in_sector = fat_offset % 512;

        let mut fat_sector = [0u8; 512];
        self.device.read_sector(self.fat_start_lba + sector, &mut fat_sector, vga_index);

        let next_byte = if offset_in_sector == 511 {
            // Next byte is in next sector
            let mut next_sector = [0u8; 512];
            self.device.read_sector(self.fat_start_lba + sector + 1, &mut next_sector, vga_index);
            next_sector[0]
        } else {
            fat_sector[offset_in_sector + 1]
        };

        let entry: u16;

        if cluster & 1 == 0 {
            // even cluster
            entry = ((next_byte as u16 & 0x0F) << 8) | (fat_sector[offset_in_sector] as u16);
        } else {
            // odd cluster
            entry = ((next_byte as u16) << 4) | ((fat_sector[offset_in_sector] as u16 & 0xF0) >> 4);
        }

        entry & 0x0FFF
    }

    pub fn list_root_dir(&self, vga_index: &mut isize) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let total_entries = self.boot_sector.root_entry_count as usize;
        let total_sectors = (total_entries * entry_size + 511) / 512;

        let mut buf = [0u8; 512];

        for sector_index in 0..total_sectors {
            self.device.read_sector(self.root_dir_start_lba + sector_index as u64, &mut buf, vga_index);

            if DEBUG {
                crate::vga::write::string(vga_index, b"Reading sector: ", crate::vga::buffer::Color::White);
                crate::vga::write::number(vga_index, self.root_dir_start_lba + sector_index as u64);
                crate::vga::write::newline(vga_index);
            }

            let entries_ptr = buf.as_ptr() as *const Entry;

            for entry_index in 0..entries_per_sector {
                let idx = sector_index * entries_per_sector + entry_index;
                if idx >= total_entries {
                    return;
                }

                let entry = unsafe { &*entries_ptr.add(entry_index) };

                // 0x00 = end of directory
                if entry.name[0] == 0x00 {
                    return;
                }

                // 0xE5 = deleted entry
                if entry.name[0] == 0xE5 {
                    continue;
                }

                // Bit 3 of attribute = Volume label
                if entry.attr & 0x08 != 0 {
                    continue;
                }

                self.print_name(entry, vga_index);

                if DEBUG {
                    self.read_file(entry.start_cluster, vga_index);
                }

                /*self.read_file(entry.start_cluster, |data| {
                  for i in 0..data.len() - 1 {
                  crate::vga::write::byte(vga_index, data[i], crate::vga::buffer::Color::Yellow);
                  }
                  }, vga_index);*/
            }
        }
    }

    fn print_name(&self, entry: &Entry, vga_index: &mut isize) {
        let mut printed_dot = false;

        crate::vga::write::string(vga_index, b"File: ", crate::vga::buffer::Color::White);

        for &b in &entry.name {
            if b == b' ' {
                break;
            }
            crate::vga::write::byte(vga_index, b, crate::vga::buffer::Color::Yellow);
        }

        for &b in &entry.ext {
            if b != b' ' && !printed_dot {
                crate::vga::write::byte(vga_index, b'.', crate::vga::buffer::Color::White);
                printed_dot = true;
            }
        }

        for &b in &entry.ext {
            if b == b' ' {
                break;
            }
            crate::vga::write::byte(vga_index, b, crate::vga::buffer::Color::Pink);
        }

        crate::vga::write::newline(vga_index);
    }

}

