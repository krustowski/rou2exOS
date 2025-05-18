pub trait BlockDevice {
    /// Reads 1 sector (usually 512 bytes) at the given LBA into `buffer`
    fn read_sector(&self, lba: u64, buffer: &mut [u8; 512]);

    /// Optional: writes 1 sector from `buffer` to `lba`
    fn write_sector(&mut self, lba: u64, buffer: &[u8; 512]);
}

static mut DISK_DATA: [u8; 1024 * 512] = [0u8; 1024 * 512]; // 1024 sectors

pub struct MemDisk {
    pub data: &'static mut [u8], // must be sector-aligned
}

impl MemDisk {
    pub fn new(data: &'static mut [u8]) -> Self {
        Self { data }
    }

    fn sector_offset(&self, lba: u64) -> usize {
        (lba as usize) * 512
    }
}

impl BlockDevice for MemDisk {
    fn read_sector(&self, lba: u64, buffer: &mut [u8; 512]) {
        let offset = self.sector_offset(lba);
        let slice = &self.data[offset..offset + 512];
        buffer.copy_from_slice(slice);
    }

    fn write_sector(&mut self, lba: u64, buffer: &[u8; 512]) {
        let offset = self.sector_offset(lba);
        let slice = &mut self.data[offset..offset + 512];
        slice.copy_from_slice(buffer);
    }
}

//
//  FLOPPY
//

pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al", 
        in("dx") port, 
        in("al") value
    );
}

pub unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    core::arch::asm!(
        "in al, dx", 
        out("al") val, 
        in("dx") port
    );
    val
}


pub fn lba_to_chs(lba: u64) -> (u8, u8, u8) {
    let sectors_per_track = 18;
    let heads = 2;

    let cylinder = (lba / (sectors_per_track * heads)) as u8;
    let temp = lba % (sectors_per_track * heads);
    let head = (temp / sectors_per_track) as u8;
    let sector = (temp % sectors_per_track + 1) as u8; // 1-based

    (cylinder, head, sector)
}

pub unsafe fn fdc_wait_irq() {
    // Wait until interrupt is fired (simulate or use actual IRQ handling)
    // For now: a naive delay loop or poll status.
    for _ in 0..100000 {
        let status = inb(0x3F4);
        if status & 0x80 != 0 {
            break;
        }
    }

    // Read floppy IRQ status
    Floppy::send_byte(0x08); // Sense interrupt
    let _st0 = Floppy::read_byte();
    let _cyl = Floppy::read_byte();
}

//
//
//

const DOR: u16 = 0x3F2;
const MSR: u16 = 0x3F4;
const FIFO: u16 = 0x3F5;

static mut DMA_BUFFER: [u8; 512] = [0u8; 512];

const DMA_CHANNEL: u8 = 2;
const DMA_ADDR_REG: u16 = 0x04; // Channel 2
const DMA_COUNT_REG: u16 = 0x05;
const DMA_PAGE_REG: u16 = 0x81;

pub unsafe fn dma_init(buffer_phys: u32, count: u16) {
    let addr = buffer_phys;
    let page = ((addr >> 16) & 0xFF) as u8;
    let offset = (addr & 0xFFFF) as u16;

    // Mask channel 2
    outb(0x0A, 0x06);

    // Reset flip-flop
    outb(0x0C, 0xFF);

    // Address (low then high)
    outb(DMA_ADDR_REG, (offset & 0xFF) as u8);
    outb(DMA_ADDR_REG, (offset >> 8) as u8);

    // Count (low then high), count - 1!
    let count = count - 1;
    outb(DMA_COUNT_REG, (count & 0xFF) as u8);
    outb(DMA_COUNT_REG, (count >> 8) as u8);

    // Page
    outb(DMA_PAGE_REG, page);

    // Unmask channel 2
    outb(0x0A, 0x02);
}

pub unsafe fn dma_set_read_mode() {
    // Mode: single transfer, address increment, read, channel 2
    outb(0x0B, 0x56);
}


//
//
//

pub struct Floppy;

fn wait_msr_ready() {
    for _ in 0..100000 {
        if unsafe { inb(MSR) } & 0x80 != 0 {
            return;
        }
    }
}

fn floppy_write_cmd(byte: u8) {
    wait_msr_ready();
    unsafe { outb(FIFO, byte) };
}

fn motor_on() {
    unsafe {
        outb(DOR, 0x1C); // Enable drive 0, motor on
    }
}

fn wait_for_data() {
    for _ in 0..1000000 {
        let st = unsafe { inb(MSR) };
        if st & 0xC0 == 0xC0 {
            return; // RQM + DIO: data ready for CPU to read
        }
    }
}

impl Floppy {
    pub fn init() {
        unsafe {
            outb(0x3F2, 0x1C); // Motor on, DMA/IRQ enabled
        }
    }

    fn send_byte(byte: u8) {
        unsafe {
            for _ in 0..100000 {
                if inb(0x3F4) & 0x80 != 0 {
                    break;
                }
            }
            outb(0x3F5, byte);
        }
    }

    fn read_byte() -> u8 {
        unsafe {
            for _ in 0..100000 {
                if inb(0x3F4) & 0x80 != 0 {
                    return inb(0x3F5);
                }
            }
            0
        }
    }

    pub fn read_sector_dma(lba: u64, buffer: &mut [u8; 512]) {
        let (c, h, s) = lba_to_chs(lba);

        unsafe {
            dma_set_read_mode();
            dma_init(DMA_BUFFER.as_ptr() as u32, 512);

            Floppy::send_byte(0x46); // READ DATA
            Floppy::send_byte((h << 2) | 0); // drive 0, head
            Floppy::send_byte(c);    // cylinder
            Floppy::send_byte(h);    // head
            Floppy::send_byte(s);    // sector (1-based)
            Floppy::send_byte(2);    // 512 = 2^2
            Floppy::send_byte(18);   // last sector
            Floppy::send_byte(0x1B); // GAP3
            Floppy::send_byte(0xFF); // DTL (don't care for 512B)

            fdc_wait_irq(); // wait for IRQ 6 (must be handled)

            // Copy from DMA buffer to output
            buffer.copy_from_slice(&DMA_BUFFER);
        }
    }


    pub fn read_sector_chs(c: u8, h: u8, s: u8, buffer: &mut [u8; 512]) {
        motor_on();

        // Send READ DATA command (0x46 = MFM, skip errors)
        floppy_write_cmd(0x46);
        floppy_write_cmd((h << 2) | 0); // Head + drive
        floppy_write_cmd(c);           // Cylinder
        floppy_write_cmd(h);           // Head
        floppy_write_cmd(s);           // Sector (1-based)
        floppy_write_cmd(2);           // 512 bytes/sector → 2^2 = 2
        floppy_write_cmd(18);          // Sectors/track
        floppy_write_cmd(0x1B);        // GAP3 length
        floppy_write_cmd(0xFF);        // DTL (not used with 512-byte sectors)

        // Poll until transfer completes
        wait_for_data();

        // For now: fill buffer with fake test data (real: map to DMA buffer)
        buffer.copy_from_slice(&[0xAB; 512]);
    }

    pub fn read_sector_chs_old(cyl: u8, head: u8, sector: u8, buffer: &mut [u8; 512]) {
        unsafe {
            // Set up DMA (or fake it if doing PIO)
            // Send READ DATA command (0x46 = MFM, multi-track, skip on error)
            Self::send_byte(0x46);
            Self::send_byte((head << 2) | 0); // Drive 0, head
            Self::send_byte(cyl);
            Self::send_byte(head);
            Self::send_byte(sector);
            Self::send_byte(2); // 512 bytes/sector -> 2^2 = 2
            Self::send_byte(18); // end of track
            Self::send_byte(0x1B); // GAP3 length
            Self::send_byte(0xFF); // DTL (unused in 512 byte sectors)

            fdc_wait_irq();

            // Read 512 bytes from FDC data port (PIO)
            for i in 0..512 {
                for _ in 0..100000 {
                    if inb(0x3F4) & 0x80 != 0 {
                        break;
                    }
                }
                if let Some(buf) = buffer.get_mut(i) {
                    *buf = inb(0x3F5);
                }
            }

            // Read 7 result bytes
            let mut result = [0u8; 7];
            for i in 0..7 {
                for _ in 0..100000 {
                    if inb(0x3F4) & 0x80 != 0 {
                        break;
                    }
                }
                if let Some(res) = result.get_mut(i) {
                    *res = inb(0x3F5);
                }
            }
        }
    }

}

impl BlockDevice for Floppy {
    fn read_sector(&self, lba: u64, buffer: &mut [u8; 512]) {
        //let (cyl, head, sector) = lba_to_chs(lba);
        //Self::read_sector_chs(cyl, head, sector, buffer);
        Self::read_sector_dma(lba, buffer);
    }

    fn write_sector(&mut self, _lba: u64, _buffer: &[u8; 512]) {
        // Optional: implement write
    }
}

