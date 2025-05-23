use crate::fs::fat12::fs::Fs;

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
    pub fn load() -> Self {
        let mut data = [0u8; FAT_SECTORS * BYTES_PER_SECTOR];
        for i in 0..FAT_SECTORS {
            read_sector(FAT_START_SECTOR + i as u16, &mut data[i * BYTES_PER_SECTOR..][..BYTES_PER_SECTOR]);
        }
        Self { data }
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
    pub fn follow_chain(&self, start: u16) -> heapless::Vec<u16, MAX_CLUSTERS> {
        let mut out = heapless::Vec::<u16, MAX_CLUSTERS>::new();
        let mut current = start;

        while let Some(next) = self.get(current) {
            if out.contains(&current) {
                break; // loop
            }

            out.push(current).ok();
            current = next;
        }

        // Push final if it's not EOC and not a loop
        if !out.contains(&current) {
            out.push(current).ok();
        }

        out
    }

    pub fn total_clusters(&self) -> usize {
        MAX_CLUSTERS
    }
}

