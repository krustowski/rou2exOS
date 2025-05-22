use crate::fs::block::BlockDevice;
use crate::fs::entry::{BootSector, Entry};
use crate::init::config::{debug_enabled, PATH, PATH_CLUSTER};
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
    pub fn read_file(&self, start_cluster: u16, sector_buf: &mut [u8; 512], vga_index: &mut isize) {
        let mut current_cluster = start_cluster;
        //let mut sector_buf = [0u8; 512];

        loop {
            let lba = self.cluster_to_lba(current_cluster);
            self.device.read_sector(lba, sector_buf, vga_index);

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

    pub fn write_file(&self, parent_cluster: u16, filename: &[u8; 11], data: &[u8], vga_index: &mut isize) {
        let entry_opt = self.find_or_create_entry(parent_cluster, filename, vga_index);
        if entry_opt.is_none() {
            crate::vga::write::string(vga_index, b"Error: no free directory entry", crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
            return;
        }

        let mut entry = entry_opt.unwrap();

        // Allocate first cluster for the file
        let first_cluster = self.allocate_cluster(vga_index);
        if first_cluster == 0 {
            crate::vga::write::string(vga_index, b"Error: no free cluster", crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
            return;
        }

        entry.start_cluster = first_cluster;
        let mut current_cluster = first_cluster;

        let mut remaining = data.len();
        let mut offset = 0;

        while remaining > 0 {
            let cluster_lba = self.cluster_to_lba(current_cluster);
            let cluster_size = 512 * self.boot_sector.sectors_per_cluster as usize;
            let to_write = core::cmp::min(cluster_size, remaining);

            if data.len() > 512 {
                return;
            }

            let mut chunk = [0u8; 512];

            if let Some(slice) = chunk.get_mut(..) {
                slice[..data.len()].copy_from_slice(data);
            }

            // Write all sectors in the cluster
            for sector in 0..self.boot_sector.sectors_per_cluster {
                self.device.write_sector(cluster_lba + sector as u64, &chunk, vga_index);
            }

            offset += to_write;
            remaining -= to_write;

            if remaining > 0 {
                let next = self.allocate_cluster(vga_index);
                if next == 0 {
                    crate::vga::write::string(vga_index, b"Error: ran out of clusters", crate::vga::buffer::Color::Red);
                    crate::vga::write::newline(vga_index);
                    return;
                }
                self.write_fat12_entry(current_cluster, next, vga_index);
                current_cluster = next;
            } else {
                self.write_fat12_entry(current_cluster, 0xFFF, vga_index); // end-of-chain
            }
        }

        entry.file_size = data.len() as u32;

        self.update_dir_entry(parent_cluster, filename, &entry, vga_index);
    }


    /// Inserts a directory entry into a directory cluster (including root).
    /// `dir_cluster == 0` means root directory.
    /// `entry_name` must be 11 bytes: 8 for name + 3 for extension.
    pub fn insert_directory_entry(&self, dir_cluster: u16, new_entry: &Entry, vga_index: &mut isize) {
        // Root directory special case
        if dir_cluster == 0 {
            let mut root_buf = [0u8; 512];

            for i in 0..(self.boot_sector.root_entry_count as usize / 16) {
                let lba =  self.root_dir_start_lba + i as u64;
                self.device.read_sector(lba, &mut root_buf, vga_index);

                for j in 0..16 {
                    let offset = j * 32;
                    if root_buf[offset] == 0x00 || root_buf[offset] == 0xE5 {
                        // Free entry
                        let entry_bytes = unsafe {
                            core::slice::from_raw_parts(
                                new_entry as *const _ as *const u8,
                                core::mem::size_of::<Entry>(),
                            )
                        };
                        root_buf[offset..offset + 32].copy_from_slice(entry_bytes);
                        self.device.write_sector(lba, &root_buf, vga_index);
                        return;
                    }
                }
            }

            // Root dir full (no expansion possible in FAT12)
            return;
        }

        // Subdirectory
        let mut cluster = dir_cluster;
        loop {
            let lba = self.cluster_to_lba(cluster);
            let mut sector_buf = [0u8; 512];
            self.device.read_sector(lba, &mut sector_buf, vga_index);

            for j in 0..16 {
                let offset = j * 32;
                if sector_buf[offset] == 0x00 || sector_buf[offset] == 0xE5 {
                    let entry_bytes = unsafe {
                        core::slice::from_raw_parts(
                            new_entry as *const _ as *const u8,
                            core::mem::size_of::<Entry>(),
                        )
                    };
                    sector_buf[offset..offset + 32].copy_from_slice(entry_bytes);
                    self.device.write_sector(lba, &sector_buf, vga_index);
                    return;
                }
            }

            // Go to next cluster in chain
            let fat_entry = self.read_fat12_entry(cluster, vga_index);
            if fat_entry >= 0xFF8 {
                // End of cluster chain, allocate a new cluster
                let next = self.allocate_cluster(vga_index);
                if next == 0 {
                    // No space left
                    return;
                }
                self.write_fat12_entry(cluster, next, vga_index);
                self.write_fat12_entry(next, 0xFFF, vga_index);

                // Clear new cluster
                let mut zero = [0u8; 512];
                self.device.write_sector(self.cluster_to_lba(next), &zero, vga_index);

                // Now insert into the new cluster
                let entry_bytes = unsafe {
                    core::slice::from_raw_parts(
                        new_entry as *const _ as *const u8,
                        core::mem::size_of::<Entry>(),
                    )
                };
                zero[0..32].copy_from_slice(entry_bytes);
                self.device.write_sector(self.cluster_to_lba(next), &zero, vga_index);
                return;
            }

            cluster = fat_entry;
        }
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

    fn update_dir_entry(&self, dir_cluster: u16, filename: &[u8; 11], updated: &Entry, vga_index: &mut isize) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let mut cluster = dir_cluster;

        while cluster < 0xFF8 {
            let cluster_lba = self.cluster_to_lba(cluster);
            let sectors = self.boot_sector.sectors_per_cluster as usize;

            for i in 0..sectors {
                let lba = cluster_lba + i as u64;
                let mut buf = [0u8; 512];
                self.device.read_sector(lba, &mut buf, vga_index);

                let entries_ptr = buf.as_mut_ptr() as *mut Entry;

                for entry_index in 0..entries_per_sector {
                    let entry = unsafe { &mut *entries_ptr.add(entry_index) };

                    if self.check_filename(entry, filename) {
                        *entry = *updated;
                        self.device.write_sector(lba, &buf, vga_index);
                        return;
                    }
                }
            }

            cluster = self.read_fat12_entry(cluster, vga_index);
        }
    }

    fn find_or_create_entry(&self, dir_cluster: u16, filename: &[u8; 11], vga_index: &mut isize) -> Option<Entry> {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let mut cluster = dir_cluster;

        while cluster < 0xFF8 {
            let cluster_lba = self.cluster_to_lba(cluster);
            let sectors = self.boot_sector.sectors_per_cluster as usize;

            for i in 0..sectors {
                let lba = cluster_lba + i as u64;
                let mut buf = [0u8; 512];
                self.device.read_sector(lba, &mut buf, vga_index);

                let entries_bytes = &mut buf[..];

                for entry_index in 0..entries_per_sector {
                    let start = entry_index * entry_size;
                    let end = start + entry_size;
                    let entry_bytes = &mut entries_bytes[start..end];

                    let entry = unsafe {
                        &*(entry_bytes.as_ptr() as *const Entry)
                    };

                    if entry.name[0] == 0x00 || entry.name[0] == 0xE5 {
                        // Free entry slot
                        let mut name: [u8; 8] = [0u8; 8];
                        let mut ext: [u8; 3] = [0u8; 3];

                        if let Some(name_slice) = filename.get(0..8) {
                            name[..name_slice.len()].copy_from_slice(name_slice);
                        }
                        if let Some(ext_slice) = filename.get(8..11) {
                            ext[..ext_slice.len()].copy_from_slice(ext_slice);
                        }

                        let new_entry = Entry {
                            name,
                            ext,
                            attr: 0x20,
                            start_cluster: 0,
                            file_size: 0,
                            ..Default::default()
                        };

                        // Write the new entry directly into the sector buffer
                        let entry_bytes = unsafe {
                            core::slice::from_raw_parts(
                                &new_entry as *const _ as *const u8,
                                entry_size
                            )
                        };

                        entries_bytes[start..end].copy_from_slice(entry_bytes);
                        self.device.write_sector(lba, &buf, vga_index);

                        return Some(new_entry);
                    }

                    if self.check_filename(entry, filename) {
                        return Some(*entry);
                    }
                }
            }

            cluster = self.read_fat12_entry(cluster, vga_index);
        }

        None
    }

    pub fn rename_file(&self, dir_cluster: u16, old_filename: &[u8; 11], new_filename: &[u8; 11], vga_index: &mut isize) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let mut sector_buf = [0u8; 512];

        if dir_cluster == 0 {
            // Root directory
            let root_dir_sector = self.root_dir_start_lba;
            let root_dir_entries = self.boot_sector.root_entry_count as usize;
            let total_sectors = (root_dir_entries * entry_size + 511) / 512;

            for i in 0..total_sectors {
                self.device.read_sector(root_dir_sector + i as u64, &mut sector_buf, vga_index);

                for entry_index in 0..entries_per_sector {
                    let offset = entry_index * entry_size;
                    let entry = unsafe { &*(sector_buf[offset..].as_ptr() as *const Entry) };

                    if entry.name[0] == 0x00 || entry.name[0] == 0xE5 {
                        continue;
                    }

                    if self.check_filename(entry, old_filename) {
                        // Found — rename it
                        sector_buf[offset..offset + 8].copy_from_slice(&new_filename[0..8]);
                        sector_buf[offset + 8..offset + 11].copy_from_slice(&new_filename[8..11]);

                        self.device.write_sector(root_dir_sector + i as u64, &sector_buf, vga_index);
                        crate::vga::write::string(vga_index, b"File renamed", crate::vga::buffer::Color::Green);
                        crate::vga::write::newline(vga_index);
                        return;
                    }
                }
            }
        } else {
            // Subdirectory
            let mut current_cluster = dir_cluster;

            loop {
                let sector_lba = self.cluster_to_lba(current_cluster);

                for sector_offset in 0..self.boot_sector.sectors_per_cluster as u64 {
                    self.device.read_sector(sector_lba + sector_offset, &mut sector_buf, vga_index);

                    for entry_index in 0..entries_per_sector {
                        let offset = entry_index * entry_size;
                        let entry = unsafe { &*(sector_buf[offset..].as_ptr() as *const Entry) };

                        if entry.name[0] == 0x00 || entry.name[0] == 0xE5 {
                            continue;
                        }

                        if self.check_filename(entry, old_filename) {
                            // Found — rename it
                            sector_buf[offset..offset + 8].copy_from_slice(&new_filename[0..8]);
                            sector_buf[offset + 8..offset + 11].copy_from_slice(&new_filename[8..11]);

                            self.device.write_sector(sector_lba + sector_offset, &sector_buf, vga_index);
                            crate::vga::write::string(vga_index, b"File renamed", crate::vga::buffer::Color::Green);
                            crate::vga::write::newline(vga_index);
                            return;
                        }
                    }
                }

                let next_cluster = self.read_fat12_entry(current_cluster, vga_index);
                if next_cluster >= 0xFF8 {
                    break;
                }

                current_cluster = next_cluster;
            }
        }

        crate::vga::write::string(vga_index, b"File not found", crate::vga::buffer::Color::Red);
        crate::vga::write::newline(vga_index);
    }

    pub fn delete_file(&self, dir_cluster: u16, filename: &[u8; 11], vga_index: &mut isize) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;

        let mut sector_buf = [0u8; 512];

        // Root directory case (cluster == 0)
        if dir_cluster == 0 {
            let root_dir_sector = self.root_dir_start_lba;
            let root_dir_entries = self.boot_sector.root_entry_count as usize;
            let total_sectors = (root_dir_entries * entry_size + 511) / 512;

            for i in 0..total_sectors {
                self.device.read_sector(root_dir_sector + i as u64, &mut sector_buf, vga_index);

                for entry_index in 0..entries_per_sector {
                    let offset = entry_index * entry_size;
                    if offset + entry_size > 512 {
                        break;
                    }

                    let entry = unsafe { &*(sector_buf[offset..].as_ptr() as *const Entry) };

                    if self.check_filename(entry, filename) {
                        sector_buf[offset] = 0xE5;
                        self.device.write_sector(root_dir_sector + i as u64, &sector_buf, vga_index);

                        crate::vga::write::string(vga_index, b"File deleted", crate::vga::buffer::Color::Green);
                        crate::vga::write::newline(vga_index);
                        return;
                    }
                }
            }

            crate::vga::write::string(vga_index, b"File not found", crate::vga::buffer::Color::Red);
            crate::vga::write::newline(vga_index);
            return;
        }

        // Subdirectory case: iterate through the directory cluster chain
        let mut current_cluster = dir_cluster;

        loop {
            let sector_lba = self.cluster_to_lba(current_cluster);

            for sector_offset in 0..self.boot_sector.sectors_per_cluster as u64 {
                self.device.read_sector(sector_lba + sector_offset, &mut sector_buf, vga_index);

                for entry_index in 0..entries_per_sector {
                    let offset = entry_index * entry_size;
                    if offset + entry_size > 512 {
                        break;
                    }

                    let entry = unsafe { &*(sector_buf[offset..].as_ptr() as *const Entry) };

                    if self.check_filename(entry, filename) {
                        sector_buf[offset] = 0xE5;
                        self.device.write_sector(sector_lba + sector_offset, &sector_buf, vga_index);

                        crate::vga::write::string(vga_index, b"File deleted", crate::vga::buffer::Color::Green);
                        crate::vga::write::newline(vga_index);
                        return;
                    }
                }
            }

            // Follow FAT chain
            let next_cluster = self.read_fat12_entry(current_cluster, vga_index);
            if next_cluster >= 0xFF8 {
                break;
            }

            current_cluster = next_cluster;
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

    pub fn create_subdirectory(&self, name: &[u8; 11], parent_cluster: u16, vga_index: &mut isize) {
        let cluster = self.allocate_cluster(vga_index);
        if cluster == 0 {
            // handle full FAT
            return;
        }

        // 1. Insert entry into current directory (e.g., root)
        let entry = Entry {
            name: name[..8].try_into().unwrap(),
            ext: name[8..].try_into().unwrap(),
            attr: 0x10,
            start_cluster: cluster,
            file_size: 0,
            ..Default::default()
        };

        // Write this entry into root dir (similar to your write_file)
        self.insert_directory_entry(parent_cluster, &entry, vga_index);

        // 2. Initialize cluster with "." and ".."
        let mut buf = [0u8; 512];

        let dot = Entry {
            name: *b".       ",
            ext: *b"   ",
            attr: 0x10,
            start_cluster: cluster,
            ..Default::default()
        };

        let dotdot = Entry {
            name: *b"..      ",
            ext: *b"   ",
            attr: 0x10,
            start_cluster: parent_cluster,
            ..Default::default()
        };

        let dot_bytes = unsafe {
            core::slice::from_raw_parts(&dot as *const _ as *const u8, 32)
        };
        buf[0..32].copy_from_slice(dot_bytes);

        let dotdot_bytes = unsafe {
            core::slice::from_raw_parts(&dotdot as *const _ as *const u8, 32)
        };
        buf[32..64].copy_from_slice(dotdot_bytes);

        self.device.write_sector(self.cluster_to_lba(cluster), &buf, vga_index);

        // 3. Update FAT
        self.write_fat12_entry(cluster, 0xFFF, vga_index);
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

            fn print_name(&self, entry: &Entry, vga_index: &mut isize) {
                let mut printed_dot = false;
                let mut file_len: usize = 0;

                crate::vga::write::string(vga_index, b" ", crate::vga::buffer::Color::White);

                for &b in &entry.name {
                    if b == b' ' {
                        break;
                    }
                    crate::vga::write::byte(vga_index, b, crate::vga::buffer::Color::Yellow);
                    file_len += 1;
                }

                for &b in &entry.ext {
                    if b != b' ' && !printed_dot {
                        crate::vga::write::byte(vga_index, b'.', crate::vga::buffer::Color::White);
                        printed_dot = true;
                        file_len += 1;
                    }
                }

                for &b in &entry.ext {
                    if b == b' ' {
                        break;
                    }
                    crate::vga::write::byte(vga_index, b, crate::vga::buffer::Color::Pink);
                    file_len += 1;
                }

                // Fill the space
                while file_len < 15 {
                    crate::vga::write::byte(vga_index, b' ', crate::vga::buffer::Color::Black);
                    file_len += 1;
                }

                if entry.attr & 0x10 != 0 {
                    crate::vga::write::string(vga_index, b"[ DIR ] => ", crate::vga::buffer::Color::Cyan);
                    crate::vga::write::number(vga_index, entry.start_cluster as u64);
                } else {
                    crate::vga::write::number(vga_index, entry.file_size as u64);
                    crate::vga::write::string(vga_index, b" bytes", crate::vga::buffer::Color::White);
                }

                crate::vga::write::newline(vga_index);
            }

        }

