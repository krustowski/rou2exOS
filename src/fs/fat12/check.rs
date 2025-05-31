use crate::fs::fat12::{entry::Entry, fs::Fs, table::FatTable, block::Floppy}; 
use crate::vga::{write::{newline, number, string}, buffer::Color};

pub struct CheckReport {
    pub errors: usize,
    pub orphan_clusters: usize,
    pub cross_linked: usize,
    pub invalid_entries: usize,
}

pub fn run_check(vga_index: &mut isize) -> CheckReport {
    let fat = FatTable::load(vga_index); // You must implement this
    let mut report = CheckReport {
        errors: 0,
        orphan_clusters: 0,
        cross_linked: 0,
        invalid_entries: 0,
    };

    let mut used_clusters = [false; 4096]; // FAT12 max: 0xFF4 (4084 entries)

    scan_directory(0, &fat, &mut used_clusters, &mut report, vga_index, 0);

    // Now check the FAT for unreferenced or multiply referenced clusters
    for cluster in 2..fat.total_clusters() {
        let fat_entry = fat.get(cluster as u16);
        if fat_entry.is_some() && !used_clusters[cluster as usize] {
            report.orphan_clusters += 1;
            continue;

            // Debug: print orphan clusters
            string(vga_index, b" -> Orphan cluster: ", Color::Yellow);
            number(vga_index, cluster as u64);
            newline(vga_index);
        }
    }

    string(vga_index, b"Done. Error count: ", Color::Green);
    number(vga_index, report.errors as u64);
    string(vga_index, b", Orphan clusters: ", Color::Green);
    number(vga_index, report.orphan_clusters as u64);
    string(vga_index, b", Cross-linked: ", Color::Green);
    number(vga_index, report.cross_linked as u64);
    string(vga_index, b", Invalid entries: ", Color::Green);
    number(vga_index, report.invalid_entries as u64);
    newline(vga_index);
    newline(vga_index);

    report
}

fn scan_directory(
    start_cluster: u16,
    fat: &FatTable,
    used: &mut [bool; 4096],
    report: &mut CheckReport,
    vga_index: &mut isize,
    depth: usize,
) {
    let floppy = Floppy;

    if depth > 64 {
        string(vga_index, b" -> Recursion depth exceeded", Color::Red);
        newline(vga_index);
        report.errors += 1;
        return;
    }

    match Fs::new(&floppy, vga_index) {
        Ok(fs) => {
            fs.for_each_entry(start_cluster, |entry| {
                if entry.name[0] == 0x00 || entry.name[0] == 0xE5 {
                    //string(vga_index, b" -> Invalid entry: ", Color::Red);
                    //number(vga_index, entry.start_cluster as u64);
                    //newline(vga_index);
                    report.invalid_entries += 1;
                    return;
                }

                if entry.attr & 0x08 != 0 {
                    // Volume label
                    return;
                }

                if entry.attr & 0x10 != 0 && entry.file_size != 0 {
                    string(vga_index, b" -> Directory has non-zero value: ", Color::Red);
                    number(vga_index, entry.start_cluster as u64);
                    newline(vga_index);
                    report.errors += 1;
                }

                if entry.start_cluster < 2 {
                    string(vga_index, b" -> Warning: entry with cluster < 2: ", Color::Yellow);
                    number(vga_index, entry.start_cluster as u64);
                    newline(vga_index);
                    return;
                }

                if entry.start_cluster == start_cluster {
                    string(vga_index, b" -> Skipping self-recursive directory", Color::Yellow);
                    newline(vga_index);
                    return;
                }

                let is_dotdot = entry.name.starts_with(b"..");
                if is_dotdot {
                    return;
                }

                //let chain = fat.follow_chain_array(entry.start_cluster);

                /*if chain.0 > chain.1.len() {
                    return;
                }

                for &cl in &chain.1[..chain.0] {
                    if cl < 2 || cl >= used.len() as u16 {
                        string(vga_index, b" -> Cluster out of bounds: ", Color::Red);
                        number(vga_index, cl as u64);
                        newline(vga_index);
                        report.errors += 1;
                        continue;
                    }

                    if used[cl as usize] {
                        string(vga_index, b" -> Cross-linked cluster: ", Color::Red);
                        number(vga_index, cl as u64);
                        newline(vga_index);
                        report.cross_linked += 1;
                    } else {
                        used[cl as usize] = true;
                    }
                }*/

                if entry.attr & 0x10 != 0 {
                    scan_directory(entry.start_cluster, fat, used, report, vga_index, depth + 1);
                } else {
                    validate_chain(entry.start_cluster, fat, used, report, vga_index);
                }
            }, &mut 0);
        }
        Err(e) => {}
    }
}

fn is_valid_name(name: &[u8; 11]) -> bool {
    name.iter().all(|&c| {
        (b'A'..=b'Z').contains(&c) ||
        (b'0'..=b'9').contains(&c) ||
        c == b' ' || b"!#$%&'()-@^_`{}~".contains(&c)
    })
}

pub fn validate_chain(
    start: u16,
    fat: &FatTable,
    used: &mut [bool; 4096],
    report: &mut CheckReport,
    vga_index: &mut isize,
) {
    let mut cluster = start;

    while fat.is_valid_cluster(cluster) && !fat.is_end_of_chain(cluster) {
        if cluster as usize >= used.len() {
            string(vga_index, b" -> Cluster out of bounds: ", Color::Red);
            number(vga_index, cluster as u64);
            newline(vga_index);
            report.errors += 1;
            return;
        }

        if used[cluster as usize] {
            string(vga_index, b" -> Cross-linked cluster: ", Color::Red);
            number(vga_index, cluster as u64);
            newline(vga_index);
            report.cross_linked += 1;
            return;
        }

        used[cluster as usize] = true;

        match fat.next_cluster(cluster) {
            Some(next) if next >= 0xFF8 => break,
            Some(next) => cluster = next,
            None => {
                string(vga_index, b" -> Invalid chain entry", Color::Red);
                newline(vga_index);
                report.errors += 1;
                break;
            }
        }
    }
}

