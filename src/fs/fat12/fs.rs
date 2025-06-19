use crate::vga::{write::{string, number, newline, byte}, buffer::Color};
use super::{block::BlockDevice, entry::{BootSector, Entry}};

/// Fs is the filesystem abstraction for FAT12 devices.
pub struct Fs<'a, D: BlockDevice> {
    pub device: &'a D,
    pub boot_sector: BootSector,
    pub fat_start_lba: u64,
    pub root_dir_start_lba: u64,
    pub data_start_lba: u64,
    pub sectors_per_cluster: u8,
}

impl<'a, D: BlockDevice> Fs<'a, D> {
    /// new method ensures the filesystem <Fs> abstraction is initilized and ready to read and
    /// write data.
    pub fn new(device: &'a D, vga_index: &mut isize) -> Result<Self, &'static str> {
        // Prepare buffer for the boot sector to load into
        let mut sector = [0u8; 512];
        device.read_sector(0, &mut sector, vga_index);

        let mut found_fat = false;

        // Search for the FAT12 label in the boot sector
        for i in 0..512 - 5 {
            if let Some(slice) = sector.get(i..i + 5) {
                if slice == b"FAT12" {
                    debugln!("Found FAT12");

                    found_fat = true;
                    break;
                }
            }
        }

        if !found_fat {
            return Err("Could not find the FAT12 label, floppy may not be present");
        }

        // Cast the sector as BootSector
        let boot_sector = unsafe { (*(sector.as_ptr() as *const BootSector)).clone() };

        debug!("OEM: ");
        debugln!(boot_sector.oem);

        debug!("Bytes/Sector: ");
        debugn!(boot_sector.bytes_per_sector);
        debugln!("");

        // Start of the FAT tables
        let fat_start = boot_sector.reserved_sectors as u64;

        // Count of sectors used for root directory
        let root_dir_sectors = ((boot_sector.root_entry_count as u32 * 32) + 511) / 512;

        // LBA address of the root directory
        let root_dir_start = fat_start + (boot_sector.fat_count as u64 * boot_sector.fat_size_16 as u64);

        // LBA address of the starting point for data
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

    /// cluster_to_lba method takes in a cluster number and returns its LBA address
    fn cluster_to_lba(&self, cluster: u16) -> u64 {
        self.data_start_lba + ((cluster as u64 - 2) * self.sectors_per_cluster as u64)
    }

    /// read_file loads the seector data into the buffer provided
    pub fn read_file(&self, start_cluster: u16, sector_buf: &mut [u8; 512], vga_index: &mut isize) {
        let mut current_cluster = start_cluster;

        loop {
            let lba = self.cluster_to_lba(current_cluster);
            self.device.read_sector(lba, sector_buf, vga_index);

            current_cluster = self.read_fat12_entry(current_cluster, vga_index);
            // Chain end
            if current_cluster >= 0xFF8 {
                break;
            }
        }
    }

    /// read_fat12_entry method reads through the FAT table to find chains of sectors used by such
    /// cluster provided
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
            // Even cluster
            entry = ((next_byte as u16 & 0x0F) << 8) | (fat_sector[offset_in_sector] as u16);
        } else {
            // Odd cluster
            entry = ((next_byte as u16) << 4) | ((fat_sector[offset_in_sector] as u16 & 0xF0) >> 4);
        }

        entry & 0x0FFF
    }

    /// write_fat12_entry writes into the FAT table according to the provided cluster number
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

    /// write_file method is a directory-agnostic function to write files into the filesystem
    pub fn write_file(
        &self,
        dir_cluster: u16,
        filename: &[u8; 11],
        data: &[u8],
        vga_index: &mut isize,
    ) {
        // If file exists, free its clusters
        if let Some((entry_lba, entry_offset, dir_entry)) =
            self.find_dir_entry_mut(dir_cluster, filename, vga_index)
        {
            let first_cluster = u16::from_le_bytes([dir_entry[26], dir_entry[27]]);
            self.free_cluster_chain(first_cluster, vga_index);

            // Mark directory entry as deleted
            let mut sector = [0u8; 512];
            self.device.read_sector(entry_lba, &mut sector, vga_index);
            sector[entry_offset] = 0xE5; // Mark as deleted
            self.device.write_sector(entry_lba, &sector, vga_index);
        }

        // Write new file
        let mut clusters_needed = (data.len() + 511) / 512;
        let first_cluster = self.allocate_cluster(vga_index);

        if first_cluster == 0 {
            debugln!("Write file: disk is full");
            string(vga_index, b"Disk is full", Color::Red);
            return;
        }

        let mut current_cluster = first_cluster;
        let mut data_offset = 0;

        while clusters_needed > 0 {
            let lba = self.cluster_to_lba(current_cluster);

            for sector_in_cluster in 0..self.boot_sector.sectors_per_cluster {
                let mut sector_data = [0u8; 512];
                let copy_len = core::cmp::min(512, data.len() - data_offset);
                if copy_len == 0 {
                    break;
                }

                if let Some(slice) = data.get(data_offset..data_offset + copy_len) {
                    sector_data[..copy_len].copy_from_slice(slice);
                }

                self.device
                    .write_sector(lba + sector_in_cluster as u64, &sector_data, vga_index);
                data_offset += copy_len;
            }

            clusters_needed -= 1;

            if clusters_needed > 0 {
                let next_cluster = self.allocate_cluster(vga_index);
                if next_cluster == 0 {
                    debugln!("Disk full mif-write, aborting");
                    string(vga_index, b"Disk full mid-write, aborting", Color::Red);
                    return;
                }

                // Write new cluster to the FAT table
                self.write_fat12_entry(current_cluster, next_cluster, vga_index);
                current_cluster = next_cluster;
            } else {
                // End of file mark
                self.write_fat12_entry(current_cluster, 0xFFF, vga_index);
            }
        }

        // Write new directory entry
        self.write_dir_entry(
            dir_cluster,
            filename,
            first_cluster,
            data.len() as u32,
            vga_index,
        );

        debugln!("Data written to a file successfully");
    }

    /// write_dir_entry method ensures a new directory entry is written into the directory file list
    fn write_dir_entry(
        &self,
        dir_cluster: u16,
        filename: &[u8; 11],
        first_cluster: u16,
        file_size: u32,
        vga_index: &mut isize,
    ) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let mut sector = [0u8; 512];

        let (start_lba, sector_count) = if dir_cluster == 0 {
            (
                self.root_dir_start_lba,
                self.boot_sector.sectors_per_cluster as usize,
            )
        } else {
            (
                self.cluster_to_lba(dir_cluster),
                self.boot_sector.sectors_per_cluster as usize,
            )
        };

        for sector_index in 0..sector_count {
            self.device.read_sector(start_lba + sector_index as u64, &mut sector, vga_index);

            for i in 0..entries_per_sector {
                let offset = i * entry_size;
                let entry = &sector[offset..offset + entry_size];

                if entry[0] == 0x00 || entry[0] == 0xE5 {
                    // Free entry — write here
                    sector[offset..offset + 11].copy_from_slice(filename);
                    sector[offset + 11] = 0x20; // file attribute: normal file
                    sector[offset + 26..offset + 28].copy_from_slice(&first_cluster.to_le_bytes());
                    sector[offset + 28..offset + 32].copy_from_slice(&file_size.to_le_bytes());

                    self.device.write_sector(start_lba + sector_index as u64, &sector, vga_index);
                    return;
                }
            }
        }

        debugln!("No directory entry slot available");
        string(vga_index, b"No dir entry slot", Color::Red);
        newline(vga_index);
    }

    fn find_dir_entry_mut(
        &self,
        dir_cluster: u16,
        filename: &[u8; 11],
        vga_index: &mut isize,
    ) -> Option<(u64, usize, [u8; 32])> {
        let entry_size = 32;
        let entries_per_sector = 512 / entry_size;
        let mut sector = [0u8; 512];

        let (start_lba, sector_count) = if dir_cluster == 0 {
            (
                self.root_dir_start_lba,
                self.boot_sector.sectors_per_cluster as usize,
            )
        } else {
            (
                self.cluster_to_lba(dir_cluster),
                self.boot_sector.sectors_per_cluster as usize,
            )
        };

        for sector_index in 0..sector_count {
            self.device.read_sector(start_lba + sector_index as u64, &mut sector, vga_index);

            for i in 0..entries_per_sector {
                let offset = i * entry_size;
                let entry = &sector[offset..offset + 11];

                if entry == filename {
                    let mut entry_buf = [0u8; 32];
                    entry_buf.copy_from_slice(&sector[offset..offset + 32]);
                    return Some((start_lba + sector_index as u64, offset, entry_buf));
                }
            }
        }

        None
    }

    /// Inserts a directory entry into a directory cluster (including root)
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

    /// Looks for an empty sector to write new data to
    fn allocate_cluster(&self, vga_index: &mut isize) -> u16 {
        let mut buf = [0u8; 512];

        for fat_index in 0..(self.boot_sector.sectors_per_cluster as u64) {
            self.device.read_sector(self.fat_start_lba + fat_index, &mut buf, vga_index);

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

    /// Ensures that previously used sectors in FAT table are freed
    fn free_cluster_chain(&self, mut cluster: u16, vga_index: &mut isize) {
        while cluster < 0xFF8 {
            let next = self.read_fat12_entry(cluster, vga_index);
            self.write_fat12_entry(cluster, 0x000, vga_index);
            cluster = next;
        }
    }

    /// Overwrites the directory entry with new data referenced by filename
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

    /// Takes in an old filename to be replaced with the new filename in the current directory
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

                        debugln!("File renamed");
                        string(vga_index, b"File renamed", Color::Green);
                        newline(vga_index);
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


                            debugln!("File renamed");
                            string(vga_index, b"File renamed", Color::Green);
                            newline(vga_index);
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

        debugln!("File not found");
        string(vga_index, b"File not found", Color::Red);
        newline(vga_index);
    }

    /// Deletes a file referenced by filename in the current directory
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

                    // Cast the read sector as an Entry
                    let entry = unsafe { &*(sector_buf[offset..].as_ptr() as *const Entry) };

                    // Check the filename
                    if self.check_filename(entry, filename) {
                        sector_buf[offset] = 0xE5;
                        self.device.write_sector(root_dir_sector + i as u64, &sector_buf, vga_index);


                        debugln!("File renamed");
                        string(vga_index, b"File deleted", Color::Green);
                        newline(vga_index);
                        return;
                    }
                }
            }

            debugln!("File not found");
            string(vga_index, b"File not found", Color::Red);
            newline(vga_index);
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

                        debugln!("File not found");
                        string(vga_index, b"File deleted", Color::Green);
                        newline(vga_index);
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

        debugln!("File not found");
        string(vga_index, b"File not found", Color::Red);
        newline(vga_index);
    }

    /// Compares given entry_name with entry name 
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

    /// Iterates over the given directory entries and provides a closure
    pub fn for_each_entry<F: FnMut(&Entry)>(&self, dir_cluster: u16, mut f: F, vga_index: &mut isize) {
        let entry_size = core::mem::size_of::<Entry>();
        let entries_per_sector = 512 / entry_size;
        let mut buf = [0u8; 512];

        let mut current_cluster = dir_cluster;

        // Root directory
        if dir_cluster == 0 {
            let total_entries = self.boot_sector.root_entry_count as usize;
            let total_sectors = (total_entries * entry_size + 511) / 512;

            // Loop over all sectors of the root directory
            for sector_index in 0..total_sectors {
                self.device.read_sector(self.root_dir_start_lba + sector_index as u64, &mut buf, vga_index);

                let entries_ptr = buf.as_ptr() as *const Entry;

                for entry_index in 0..entries_per_sector {
                    if sector_index * entries_per_sector + entry_index >= total_entries {
                        return;
                    }

                    // Cast the entry_index as an Entry
                    let entry = unsafe { &*entries_ptr.add(entry_index) };

                    // Propagate the entry into the closure
                    f(entry);
                }
            }
            // Generic subdirectory
        } else {

            let sector_start = self.cluster_to_lba(current_cluster);

            for i in 0..self.boot_sector.sectors_per_cluster {
                self.device.read_sector(sector_start as u64 + i as u64, &mut buf, vga_index);

                let entries_ptr = buf.as_ptr() as *const Entry;

                for entry_index in 0..entries_per_sector {
                    // Cast the entry_index as an Entry
                    let entry = unsafe { &*entries_ptr.add(entry_index) };

                    // Propagate the entry into the closure
                    f(entry);
                }

                // Read next cluster number
                current_cluster = self.read_fat12_entry(current_cluster, vga_index);
                if current_cluster >= 0xFF8 {
                    break;
                }
            }
        }
    }

    /// Creates a new directory (folder) in the given scope/directory
    pub fn create_subdirectory(&self, name: &[u8; 11], parent_cluster: u16, vga_index: &mut isize) {
        let cluster = self.allocate_cluster(vga_index);
        if cluster == 0 {
            // Handle full FAT
            return;
        }

        // Prepare the Entry to be inserted into the current directory
        let entry = Entry {
            name: name[..8].try_into().unwrap(),
            ext: name[8..].try_into().unwrap(),
            attr: 0x10,
            start_cluster: cluster,
            file_size: 0,
            ..Default::default()
        };

        self.insert_directory_entry(parent_cluster, &entry, vga_index);

        // Initialize cluster with "." and ".." entries
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

        // Serialize the entry into the byte slice
        let dot_bytes = unsafe {
            core::slice::from_raw_parts(&dot as *const _ as *const u8, 32)
        };
        buf[0..32].copy_from_slice(dot_bytes);

        let dotdot_bytes = unsafe {
            core::slice::from_raw_parts(&dotdot as *const _ as *const u8, 32)
        };
        buf[32..64].copy_from_slice(dotdot_bytes);

        // Write dot and dotdot entries to a new directory
        self.device.write_sector(self.cluster_to_lba(cluster), &buf, vga_index);

        // Update the FAT table
        self.write_fat12_entry(cluster, 0xFFF, vga_index);

        debugln!("Created a subdirectory");
    }

    /// Lists all entries of a given directory
    pub fn list_dir(&self, start_cluster: u16, entry_name: &[u8; 11], vga_index: &mut isize) -> isize {
        let mut status: isize = 0;

        self.for_each_entry(start_cluster, | entry | {
            if entry.name[0] == 0x00 {
                status = -1;
                return;
            }

            if entry.name[0] == 0xE5 || entry.attr & 0x08 != 0 {
                return;
            }

            if self.check_filename(entry, entry_name) {
                status = entry.start_cluster as isize;
                return;
            }

            if entry_name[0] == b' ' {
                self.print_name(entry, vga_index);
            }
        }, &mut 0);

        status
    }

    fn print_name(&self, entry: &Entry, vga_index: &mut isize) {
        let mut printed_dot = false;
        let mut file_len: usize = 0;

        string(vga_index, b" ", Color::White);

        for &b in &entry.name {
            if b == b' ' {
                break;
            }
            byte(vga_index, b, Color::Yellow);
            file_len += 1;
        }

        for &b in &entry.ext {
            if b != b' ' && !printed_dot {
                byte(vga_index, b'.', Color::White);
                printed_dot = true;
                file_len += 1;
            }
        }

        for &b in &entry.ext {
            if b == b' ' {
                break;
            }
            byte(vga_index, b, Color::Pink);
            file_len += 1;
        }

        // Fill the space
        while file_len < 15 {
            byte(vga_index, b' ', Color::Black);
            file_len += 1;
        }

        if entry.attr & 0x10 != 0 {
            string(vga_index, b"[ DIR ] => ", Color::Cyan);
            number(vga_index, entry.start_cluster as u64);
        } else {
            number(vga_index, entry.file_size as u64);
            string(vga_index, b" bytes", Color::White);
        }

        newline(vga_index);
    }

}

