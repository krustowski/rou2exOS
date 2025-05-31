use crate::fs::fat12::{block::Floppy, fs::Fs};

use super::block::BlockDevice;

/// The number of bytes in a sector.
const BYTES_PER_SECTOR: usize = 512;

/// The size of the FAT table in sectors (for a 1.44MB floppy, usually 9)
const FAT_SECTORS: usize = 9;

/// The offset (in sectors) where the FAT starts (after boot sector)
const FAT_START_SECTOR: u16 = 1;

/// Total number of clusters in FAT12 (maximum 4084)
const MAX_CLUSTERS: usize = 4085;

/// End-of-chain marker for FAT12 (>= 0xFF8)
const FAT12_EOC_MIN: u16 = 0x0FF8;

pub struct FatTable {
    /// Raw FAT bytes (only the first copy for now)
    data: [u8; FAT_SECTORS * BYTES_PER_SECTOR],
}

impl FatTable {
    pub fn load(vga_index: &mut isize) -> Self {
        let mut data = [0u8; FAT_SECTORS * BYTES_PER_SECTOR];

        let floppy = Floppy;
        let mut buf: [u8; 512] = [0u8; BYTES_PER_SECTOR];

        match Fs::new(&floppy, vga_index) {
            Ok(fs) => {
        for i in 0..FAT_SECTORS {

            //fs.device.read_sector(FAT_START_SECTOR + i as u16, &mut data[i * BYTES_PER_SECTOR..][..BYTES_PER_SECTOR], vga_index);
            fs.device.read_sector((FAT_START_SECTOR + i as u16) as u64, &mut buf, vga_index);
            data[i * BYTES_PER_SECTOR..(i + 1) * BYTES_PER_SECTOR].copy_from_slice(&buf);
        }
        Self { data }
            }
            Err(e) => {
                Self { data: [0u8; FAT_SECTORS * BYTES_PER_SECTOR] }
            }
        }

    }

    /// Get the next cluster in the chain.
    /// Returns `None` for free cluster or end-of-chain.
    pub fn get(&self, cluster: u16) -> Option<u16> {
        if cluster < 2 || cluster >= MAX_CLUSTERS as u16 {
            return None;
        }

        let index = (cluster as usize * 3) / 2;
        let b1 = self.data.get(index).copied().unwrap_or(0);
        let b2 = self.data.get(index + 1).copied().unwrap_or(0);
        let raw = if cluster & 1 == 0 {
            ((b2 as u16 & 0x0F) << 8) | b1 as u16
        } else {
            ((b2 as u16) << 4) | ((b1 as u16 & 0xF0) >> 4)
        };

        if raw >= FAT12_EOC_MIN {
            None
        } else {
            Some(raw)
        }
    }

    /// Follow a cluster chain until end-of-chain or loop.
    pub fn follow_chain_array(&self, start: u16) -> (usize, [u16; MAX_CLUSTERS]) {
        let mut result = [0u16; MAX_CLUSTERS];
        let mut len = 0;
        let mut current = start;

        while let Some(next) = self.get(current) {
            // skip cluster 0 and 1 which are reserved in FAT12
            if current < 2 || current >= MAX_CLUSTERS as u16 {
                break;
            }

            // loop protection
            if result[..len].contains(&current) {
                break;
            }

            result[len] = current;
            len += 1;

            if len >= MAX_CLUSTERS {
                break;
            }

            current = next;
        }

        // include last if valid and not already included
        if current >= 2 && !result[..len].contains(&current) && len < MAX_CLUSTERS {
            result[len] = current;
            len += 1;
        }

        (len, result)
    }

    pub fn total_clusters(&self) -> usize {
        MAX_CLUSTERS
    }

    pub fn next_cluster(&self, cluster: u16) -> Option<u16> {
        if cluster < 2 || cluster >= 0xFF8 {
            return None;
        }

        let offset = (cluster as usize * 3) / 2;
        if offset + 1 >= self.data.len() {
            return None;
        }

        let val = if cluster & 1 == 0 {
            // Even cluster
            ((self.data[offset] as u16) | ((self.data[offset + 1] as u16) << 8)) & 0x0FFF
        } else {
            // Odd cluster
            ((self.data[offset] as u16) >> 4 | ((self.data[offset + 1] as u16) << 4)) & 0x0FFF
        };

        Some(val)
    }

    pub fn is_valid_cluster(&self, cluster: u16) -> bool {
        (2..=0xFEF).contains(&cluster)
    }

    pub fn is_end_of_chain(&self, cluster: u16) -> bool {
        (0xFF8..=0xFFF).contains(&cluster)
    }
}

