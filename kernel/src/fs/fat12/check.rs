use crate::fs::{entry::Entry, fat12::read_sector}; 

pub struct CheckReport {
    pub errors: usize,
    pub orphan_clusters: usize,
    pub cross_linked: usize,
    pub invalid_entries: usize,
}

pub fn run_check() -> CheckReport {
    let fat = FatTable::load(); // You must implement this
    let mut report = CheckReport {
        errors: 0,
        orphan_clusters: 0,
        cross_linked: 0,
        invalid_entries: 0,
    };

    let mut used_clusters = [false; 4096]; // FAT12 max: 0xFF4 (4084 entries)

    print!("[check_fat12] Scanning root directory...\n");

    scan_directory(0, &fat, &mut used_clusters, &mut report);

    // Now check the FAT for unreferenced or multiply referenced clusters
    for cluster in 2..fat.total_clusters() {
        let fat_entry = fat.get(cluster);
        if fat_entry.is_some() && !used_clusters[cluster as usize] {
            print!("  -> Orphan cluster: {}\n", cluster);
            report.orphan_clusters += 1;
        }
    }

    print!("[check_fat12] Done. Errors: {}\n", report.errors);
    report
}

fn scan_directory(
    start_cluster: Cluster,
    fat: &FatTable,
    used: &mut [bool; 4096],
    report: &mut CheckReport,
) {
    let entries = crate::fs::fat12::read_directory(start_cluster);
    for entry in entries {
        if entry.is_deleted() {
            continue;
        }

        if !entry.is_valid() {
            report.invalid_entries += 1;
            print!("  -> Invalid entry: {}\n", entry.name_str());
            continue;
        }

        // Validate cluster chain
        let chain = fat.follow_chain(entry.first_cluster);
        for &cl in &chain {
            if cl >= used.len() as u16 {
                print!("  -> Cluster out of bounds: {}\n", cl);
                report.errors += 1;
                continue;
            }

            if used[cl as usize] {
                print!("  -> Cross-linked cluster: {}\n", cl);
                report.cross_linked += 1;
            } else {
                used[cl as usize] = true;
            }
        }

        if entry.is_directory() {
            scan_directory(entry.first_cluster, fat, used, report);
        }
    }
}

