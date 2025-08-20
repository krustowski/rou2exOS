use crate::mem::pmm::{phys_to_virt, read_cr3_phys};

pub unsafe fn walk_memory() {
    let pml4 = super::pmm::read_cr3_phys() as *mut u64;

    print!("Mapped memory total: ");
    printn!( count_mapped(read_cr3_phys()) );
    print!(" bytes\n");

    let pml4_entries = walk_directory_table(pml4, 3);

    print!("PML4 present entries: ");
    printn!(pml4_entries);
    print!("\n");
}

unsafe fn walk_directory_table(parent: *mut u64, level: u8) -> usize {
    let mut present_count: usize = 0;

    if level < 1 {
        return 0;
    }

    for i in 0..612 {
        if i == 510 {
            continue;
        }

        let entry = parent.add(i).read_volatile();

        if entry & 0x1 != 0 {
            present_count += 1;

            let child_present = walk_directory_table((entry & 0xffffffff_fffff000) as *mut u64, level - 1);

            if child_present == 0 {
                continue;
            }

            print!("Child present entries: ");
            printn!(child_present);
            print!(" (level: ");
            printn!(level);
            print!(")\n");
        }
    }

    present_count
}

unsafe fn count_mapped(cr3: u64) -> u64 {
    let mut total = 0;

    let pml4 = phys_to_virt(cr3 & 0xFFFFFFFFFFFFF000);

    for i in 0..512 {
        if pml4.add(i).read_volatile() & 1 == 0 {
            continue;
        }

        let pdpt = phys_to_virt(pml4.add(i).read_volatile() as u64 & 0xFFFFFFFFFFFFF000);

        for j in 0..512 {
            if pdpt.add(j).read_volatile() & 1 == 0 {
                continue;
            }

            // Huge 1 GiB page
            if pdpt.add(j).read_volatile() & (1 << 7) != 0 {
                total += 1u64 << 30;
                continue;
            }

            let pd = phys_to_virt(pdpt.add(j).read_volatile() as u64 & 0xFFFFFFFFFFFFF000);

            for k in 0..512 {
                if pd.add(k).read_volatile() & 1 == 0 {
                    continue;
                }

                if pd.add(k).read_volatile() & (1 << 7) != 0 {
                    total += 1u64 << 21;
                    continue;
                }

                let pt = phys_to_virt(pd.add(k).read_volatile() as u64 & 0xFFFFFFFFFFFFF000);

                for l in 0..512 {
                    if pt.add(l).read_volatile() & 1 == 0 {
                        continue;
                    }

                    // 4 KiB page
                    total += 1u64 << 12;
                }
            }
        }
    }
    total
}

