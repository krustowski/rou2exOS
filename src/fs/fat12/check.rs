use crate::fs::fat12::{fs::Filesystem, table::FatTable, block::Floppy}; 

pub struct CheckReport {
    pub errors: usize,
    pub orphan_clusters: usize,
    pub cross_linked: usize,
    pub invalid_entries: usize,
}

pub fn run_check() -> CheckReport {
    let fat = FatTable::load();
    let mut report = CheckReport {
        errors: 0,
        orphan_clusters: 0,
        cross_linked: 0,
        invalid_entries: 0,
    };

    let mut used_clusters = [false; 4096]; // FAT12 max: 0xFF4 (4084 entries)

    scan_directory(0, &fat, &mut used_clusters, &mut report, 0);

    // Check the FAT for unreferenced or multiply referenced clusters
    report.orphan_clusters += used_clusters.iter().zip((0..).map(|cluster| fat.get(cluster))).skip(2).filter_map(|x| if !x.0 { x.1 } else {None}).count();

    /*print!("Done. Error count: ");
    printn!(report.errors as u64);
    print!(", Orphan clusters: ");
    printn!(report.orphan_clusters as u64);
    print!(", Cross-linked: ");
    printn!(report.cross_linked as u64);
    print!(", Invalid entries: ");
    printn!(report.invalid_entries as u64);
    println!();
    println!();*/

    report
}

fn scan_directory(
    start_cluster: u16,
    fat: &FatTable,
    used: &mut [bool; 4096],
    report: &mut CheckReport,
    depth: usize,
) {
    if depth > 64 {
        error!(" -> Recursion depth exceeded");
        println!();

        report.errors += 1;
        return;
    }

    let floppy = Floppy::init();

    if let Ok(fs) = Filesystem::new(&floppy) {
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
                error!(" -> Directory has non-zero value: ");
                printn!(entry.start_cluster as u64);
                println!();

                report.errors += 1;
            }

            if entry.start_cluster < 2 {
                warn!(" -> Warning: entry with cluster < 2: ");
                printn!(entry.start_cluster as u64);
                println!();
                return;
            }

            if entry.start_cluster == start_cluster {
                warn!(" -> Skipping self-recursive directory");
                println!();
                return;
            }

            let is_dotdot = entry.name.starts_with(b"..");
            if is_dotdot {
                return;
            }

            if entry.attr & 0x10 != 0 {
                scan_directory(entry.start_cluster, fat, used, report, depth + 1);
            } else {
                validate_chain(entry.start_cluster, fat, used, report);
            }
        });
    }
}

fn is_valid_name(name: &[u8; 11]) -> bool {
    name.iter().all(|&c| {
        c.is_ascii_uppercase() ||
        c.is_ascii_digit() ||
        c == b' ' || b"!#$%&'()-@^_`{}~".contains(&c)
    })
}

pub fn validate_chain(
    start: u16,
    fat: &FatTable,
    used: &mut [bool; 4096],
    report: &mut CheckReport,
) {
    let mut cluster = start;

    while fat.is_valid_cluster(cluster) && !fat.is_end_of_chain(cluster) {
        if cluster as usize >= used.len() {
            error!(" -> Cluster out of bounds: ");
            printn!(cluster as u64);
            println!();

            report.errors += 1;
            return;
        }

        if used[cluster as usize] {
            error!(" -> Cross-linked cluster: ");
            printn!(cluster as u64);

            report.cross_linked += 1;
            return;
        }

        used[cluster as usize] = true;

        match fat.next_cluster(cluster) {
            Some(next) if next >= 0xFF8 => break,
            Some(next) => cluster = next,
            None => {
                error!(" -> Invalid chain entry");
                println!();

                report.errors += 1;
                break;
            }
        }
    }
}

