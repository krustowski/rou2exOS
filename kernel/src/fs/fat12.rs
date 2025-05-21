use core::ptr::eq;

use crate::fs::block::BlockDevice;
use crate::fs::entry::{BootSector, Entry};
use crate::init::config::debug_enabled;
use crate::init::config::get_path;

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
                if debug_enabled() {
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

        if debug_enabled() {
            crate::vga::write::string(vga_index, b"OEM: ", crate::vga::buffer::Color::White);
            for b in &boot_sector.oem {
                crate::vga::write::byte(vga_index, *b, crate::vga::buffer::Color::Green);
            }
            crate::vga::write::newline(vga_index);
        }

        if debug_enabled() {
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
    pub fn read_file(&self, start_cluster: u16, vga_index: &mut isize) {
        let mut current_cluster = start_cluster;
        let mut sector_buf = [0u8; 512];

        loop {
            let lba = self.cluster_to_lba(current_cluster);
            self.device.read_sector(lba, &mut sector_buf, vga_index);

            //if debug_enabled() {
            crate::vga::write::string(vga_index, &sector_buf, crate::vga::buffer::Color::Yellow);
            //}

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

    pub fn write_fat12_entry(&self, cluster: u16, value: u16, vga_index: &mut isize) {
        let fat_offset = (cluster as usize * 3) / 2;
        let fat_sector = self.fat_start_lba + (fat_offset / 512) as u64;

        let mut buf = [0u8; 512];
        self.device.read_sector(fat_sector, &mut buf, vga_index);

        if cluster & 1 == 0 {
            buf[fat_offset % 512] = (value & 0xFF) as u8;
            buf[(fat_offset + 1) % 512] = ((buf[(fat_offset + 1) % 512] & 0xF0) | ((value >> 8) as u8 & 0x0F));
        } else {
            buf[fat_offset % 512] = ((buf[fat_offset % 512] & 0x0F) | ((value << 4) as u8 & 0xF0));
            buf[(fat_offset + 1) % 512] = ((value >> 4) & 0xFF) as u8;
        }

        self.device.write_sector(fat_sector, &buf, vga_index);
    }

    pub fn write_file(&self, filename: &[u8; 11], data: &[u8], vga_index: &mut isize) {
        let entry = self.find_or_create_entry(filename, vga_index);
        if entry.is_none() {
            crate::vga::write::string(vga_index, b"Error: no free directory entry\n", crate::vga::buffer::Color::Red);
            return;
        }

        let mut entry = entry.unwrap();

        // Allocate clusters and write data
        let mut cluster = self.allocate_cluster(vga_index);
        let mut remaining = data.len();
        let mut offset: usize = 0;

        crate::vga::write::string(vga_index, b"Dost", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);

        entry.start_cluster = cluster;
        let mut current_cluster = cluster;

        while remaining > 0 {
            let cluster_lba = self.cluster_to_lba(current_cluster);
            let to_write = core::cmp::min(512 * self.boot_sector.sectors_per_cluster as usize, remaining);

            let mut chunk: [u8; 512] = [0u8; 512];

            if data.len() > 512 {
                return;
            }

            if let Some(slice) = chunk.get_mut(..) {
                slice[..data.len()].copy_from_slice(data);
            }

            let sectors = self.boot_sector.sectors_per_cluster as usize;

            // sectors
            for i in 0..sectors {
                /*let sector_offset = i * 512;
                let slice = &cluster_data[sector_offset..sector_offset + 512];
                let sector_data: &[u8; 512] = slice.try_into().expect("Sector conversion error");*/

            crate::vga::write::string(vga_index, b"Writing data...", crate::vga::buffer::Color::Yellow);
            crate::vga::write::newline(vga_index);

                self.device.write_sector(
                    cluster_lba + i as u64, 
                    &chunk, 
                    vga_index,
                );

            }

            crate::vga::write::string(vga_index, b"Boha pica uz", crate::vga::buffer::Color::Yellow);
            crate::vga::write::newline(vga_index);

            remaining -= to_write;
            offset += to_write;

            if remaining > 0 {
                let next_cluster = self.allocate_cluster(vga_index);
                self.write_fat12_entry(current_cluster, next_cluster, vga_index);
                current_cluster = next_cluster;
            } else {
                self.write_fat12_entry(current_cluster, 0xFFF, vga_index); // end-of-chain
            }
        }

        // Update directory entry (e.g., size, start_cluster, etc.)
        entry.file_size = data.len() as u32;
        self.update_dir_entry(filename, &entry, vga_index);
    }

    fn allocate_cluster(&self, vga_index: &mut isize) -> u16 {
        let mut buf = [0u8; 512];

        for fat_index in 0..(self.boot_sector.sectors_per_cluster as u64) {
            self.device.read_sector(self.fat_start_lba + fat_index, &mut buf, vga_index);

            crate::vga::write::string(vga_index, b"Jezisi", crate::vga::buffer::Color::Yellow);
            crate::vga::write::newline(vga_index);

            for cluster in 2..(self.boot_sector.total_sectors_16) {
                let value = self.read_fat12_entry(cluster, vga_index);
                if value == 0x000 {
                    self.write_fat12_entry(cluster, 0xFFF, vga_index);
                    return cluster;
                }
            }
        }

        //panic!("FAT is full");
        0
    }

    fn update_dir_entry(&self, filename: &[u8; 11], updated: &Entry, vga_index: &mut isize) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let total_entries = self.boot_sector.root_entry_count as usize;
        let total_sectors = (total_entries * entry_size + 511) / 512;

        let mut buf = [0u8; 512];

        for sector_index in 0..total_sectors {
            self.device.read_sector(self.root_dir_start_lba + sector_index as u64, &mut buf, vga_index);
            let entries_ptr = buf.as_mut_ptr() as *mut Entry;

            for entry_index in 0..entries_per_sector {
                let entry = unsafe { &mut *entries_ptr.add(entry_index) };

                if self.check_filename(entry, filename) {
                    *entry = *updated;
                    self.device.write_sector(self.root_dir_start_lba + sector_index as u64, &buf, vga_index);
                    return;
                }
            }
        }
    }


    fn find_or_create_entry(&self, filename: &[u8; 11], vga_index: &mut isize) -> Option<Entry> {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let total_entries = self.boot_sector.root_entry_count as usize;
        let total_sectors = (total_entries * entry_size + 511) / 512;

        let mut buf = [0u8; 512];

        for sector_index in 0..total_sectors {
            self.device.read_sector(self.root_dir_start_lba + sector_index as u64, &mut buf, vga_index);
            //let entries_ptr = buf.as_ptr() as *const Entry;

            let entries_bytes = &buf[..];

            for entry_index in 0..entries_per_sector {
                let start = entry_index * entry_size;
                let end = start + entry_size;
                let entry_bytes = &entries_bytes[start..end];

                let entry = unsafe {
                    &*(entry_bytes.as_ptr() as *const Entry)
                };

                //let entry = unsafe { &*entries_ptr.add(entry_index) };

                if entry.name[0] == 0x00 || entry.name[0] == 0xE5 {
                    // free entry
                    crate::vga::write::string(vga_index, b"Vypadni", crate::vga::buffer::Color::Yellow);
                    crate::vga::write::newline(vga_index);

                    let mut name: [u8; 8] = [0u8; 8];
                      let mut ext: [u8; 3] = [0u8; 3];

                      if let Some(name_slice) = filename.get(0..8) {
                      name[..name_slice.len()].copy_from_slice(name_slice);
                      }
                      if let Some(ext_slice) = filename.get(8..11) {
                      ext[..ext_slice.len()].copy_from_slice(ext_slice);
                      }

                    return Some(Entry {
                        name: name,
                        ext: ext,
                        attr: 0x20,
                        start_cluster: 0,
                        file_size: 0,
                        ..Default::default()
                    });
                }

                if self.check_filename(entry, filename) {
                    return Some(*entry);
                }
            }
        }

        None
    }

    pub fn rename_file(&self, old_filename: &[u8; 11], new_filename: &[u8; 11], vga_index: &mut isize) {
        let root_dir_sector = self.boot_sector.reserved_sectors as u64
            + (2 as u64 * 9 as u64);
        let root_dir_entries = (self.boot_sector.root_entry_count as usize * 32) / 512;

        for i in 0..root_dir_entries {
            let mut sector: [u8; 512] = [0; 512];
            self.device.read_sector(root_dir_sector + i as u64, &mut sector, vga_index);

            let entries_bytes = &sector[..];

            for entry_index in 0..(512 / 32) {
                let offset = entry_index * 32;

                let start = entry_index * 32;
                let end = start + 32;
                let entry_bytes = &entries_bytes[start..end];

                let entry = unsafe {
                    &*(entry_bytes.as_ptr() as *const Entry)
                };

                //let offset = entry_index * 32;
                //let entry = sector[offset..offset + 32].as_ptr() as *const Entry;

                if entry.name[0] == 0x00 {
                    // No more files
                    continue;
                }

                if entry.name[0] == 0xE5 {
                    // Deleted file
                    continue;
                }
                    
                crate::vga::write::string(vga_index, b"Current: ", crate::vga::buffer::Color::Yellow);
                crate::vga::write::string(vga_index, &entry.name, crate::vga::buffer::Color::Yellow);
                crate::vga::write::string(vga_index, b", Old: ", crate::vga::buffer::Color::Yellow);
                crate::vga::write::string(vga_index, old_filename, crate::vga::buffer::Color::Yellow);
                crate::vga::write::string(vga_index, b", New: ", crate::vga::buffer::Color::Yellow);
                crate::vga::write::string(vga_index, new_filename, crate::vga::buffer::Color::Yellow);
                crate::vga::write::newline(vga_index);

                if self.check_filename(entry, old_filename) {
                    // Found the file — rename it
                    let mut new_entry = [0u8; 32];
                    new_entry.copy_from_slice(entry_bytes);

                    // Set new name
                    new_entry[0..8].copy_from_slice(&new_filename[0..8]);
                    new_entry[8..11].copy_from_slice(b"TXT");

                    // Write back into the sector
                    for j in 0..32 {
                        sector[offset + j] = new_entry[j];
                    }

                    self.device.write_sector(root_dir_sector + i as u64, &sector, vga_index);

                    crate::vga::write::string(vga_index, b"File renamed", crate::vga::buffer::Color::Green);
                    crate::vga::write::newline(vga_index);
                    return;
                }
            }
        }

        crate::vga::write::string(vga_index, b"File not found", crate::vga::buffer::Color::Red);
        crate::vga::write::newline(vga_index);
    }

    pub fn delete_file(&self, filename: &[u8; 11], vga_index: &mut isize) {
        let root_dir_sector = self.boot_sector.reserved_sectors as u64 + (2 * 9); // 2 FATs × 9 sectors per FAT
        let root_dir_entries = (self.boot_sector.root_entry_count as usize * 32) / 512;

        for i in 0..root_dir_entries {
            let mut sector: [u8; 512] = [0; 512];
            self.device.read_sector(root_dir_sector + i as u64, &mut sector, vga_index);

            for entry_index in 0..(512 / 32) {
                let offset = entry_index * 32;
                let entry_bytes = &sector[offset..offset + 32];

                let entry = unsafe {
                    &*(entry_bytes.as_ptr() as *const Entry)
                };

                if !self.check_filename(entry, filename) {
                    continue;
                }

                // Found the file. Mark it as deleted by setting first byte to 0xE5
                sector[offset] = 0xE5;

                self.device.write_sector(root_dir_sector + i as u64, &sector, vga_index);

                crate::vga::write::string(vga_index, b"File deleted", crate::vga::buffer::Color::Green);
                crate::vga::write::newline(vga_index);
                return;
            }
        }

        crate::vga::write::string(vga_index, b"File not found", crate::vga::buffer::Color::Red);
        crate::vga::write::newline(vga_index);
    }

    fn check_filename(&self, entry: &Entry, entry_name: &[u8; 11]) -> bool {
        if entry.name.len() != 8 {
            return false;
        }

        for i in 0..8 {
            if entry.name[i] != entry_name[i] {
                return false;
            }
        }
        true
    }

    fn _check_filename(&self, entry: &Entry, entry_name: &[u8]) -> bool {
        let entry_len = entry_name.len();
        let mut equal = true;

        if entry_len <= 11 && entry_len > 0 {
            for i in 0..entry.name.len() {
                if let Some(org) = entry.name.get(i) {
                    if let Some(query) = entry_name.get(i) {

                        // Filename end
                        /*if *org == 0 {
                          break;
                          }*/

                        if *query - 32 != *org {
                            equal = false;
                            break;
                        }

                        if i == entry_len - 1 {
                            if let Some(org_nxt) = entry.name.get(i + 1) {
                                if *org_nxt != b' ' {
                                    equal = false;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        equal
    }

    pub fn list_dir(&self, start_cluster: u16, entry_name: &[u8], vga_index: &mut isize) -> isize {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let mut buf = [0u8; 512];

        let mut current_cluster = start_cluster;

        if start_cluster == 0 {
            let total_entries = self.boot_sector.root_entry_count as usize;
            let total_sectors = (total_entries * entry_size + 511) / 512;

            for sector_index in 0..total_sectors {
                self.device.read_sector(self.root_dir_start_lba + sector_index as u64, &mut buf, vga_index);

                let entries_ptr = buf.as_ptr() as *const Entry;
                for entry_index in 0..entries_per_sector {
                    if sector_index * entries_per_sector + entry_index >= total_entries {
                        return -1;
                    }

                    let entry = unsafe { &*entries_ptr.add(entry_index) };

                    if entry.name[0] == 0x00 {
                        return -1;
                    }

                    if entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 {
                        continue;
                    }

                    if entry_name.len() > 11 {
                        continue;
                    }

                    let mut name: [u8; 11] = [b' '; 11];
                    name[..entry_name.len()].copy_from_slice(entry_name);
                    name[8..11].copy_from_slice(b"TXT");

                    if entry_name.len() > 0 {
                        // TODO this is not safe as this allows to CD into file sectors!
                        //if self.check_filename(entry, entry_name) && entry.attr & 0x10 != 0 {
                        if self.check_filename(entry, &name) {
                            return entry.start_cluster as isize;
                        }
                    } else {
                        self.print_name(entry, vga_index);
                    }
                    }
                }
            } else {
                loop {
                    let sector_start = self.cluster_to_lba(current_cluster);

                    for i in 0..self.boot_sector.sectors_per_cluster {
                        self.device.read_sector(sector_start as u64 + i as u64, &mut buf, vga_index);

                        let entries_ptr = buf.as_ptr() as *const Entry;
                        for entry_index in 0..entries_per_sector {
                            let entry = unsafe { &*entries_ptr.add(entry_index) };

                            if entry.name[0] == 0x00 {
                                return -1;
                            }

                            if entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 {
                                continue;
                            }

                            if entry_name.len() > 11 {
                                continue;
                            }

                            let mut name: [u8; 11] = [b' '; 11];
                            name[..entry_name.len()].copy_from_slice(entry_name);
                            name[8..11].copy_from_slice(b"TXT");

                            if entry_name.len() > 0 {
                                // TODO this is not safe as this allows to CD into file sectors!
                                //if self.check_filename(entry, entry_name) && entry.attr & 0x10 != 0 {
                                if self.check_filename(entry, &name) {
                                    return entry.start_cluster as isize;
                                }
                            } else {
                                self.print_name(entry, vga_index);
                            }
                            }
                        }

                        current_cluster = self.read_fat12_entry(current_cluster, vga_index);
                        if current_cluster >= 0xFF8 {
                            break;
                        }
                    }
                }
                0
            }

            pub fn list_root_dir(&self, vga_index: &mut isize) {
                let entry_size = core::mem::size_of::<Entry>();
                let entries_per_sector = 512 / entry_size;
                let total_entries = self.boot_sector.root_entry_count as usize;
                let total_sectors = (total_entries * entry_size + 511) / 512;

                let mut buf = [0u8; 512];

                for sector_index in 0..total_sectors {
                    self.device.read_sector(self.root_dir_start_lba + sector_index as u64, &mut buf, vga_index);

                    if debug_enabled() {
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

                        /*if debug_enabled() {
                          self.read_file(entry.start_cluster, vga_index);
                          }*/
                    }
                }
            }

            fn print_name(&self, entry: &Entry, vga_index: &mut isize) {
                let mut printed_dot = false;

                crate::vga::write::string(vga_index, b" ", crate::vga::buffer::Color::White);

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

                if entry.attr & 0x10 != 0 {
                    crate::vga::write::string(vga_index, b" => DIR => ", crate::vga::buffer::Color::White);
                    crate::vga::write::number(vga_index, entry.start_cluster as u64);
                } else {
                    crate::vga::write::string(vga_index, b" (", crate::vga::buffer::Color::White);
                    crate::vga::write::number(vga_index, entry.file_size as u64);
                    crate::vga::write::string(vga_index, b" bytes)", crate::vga::buffer::Color::White);
                }

                crate::vga::write::newline(vga_index);
            }

        }

