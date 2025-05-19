use crate::vga::{self, buffer};

pub trait BlockDevice {
    /// Reads 1 sector (usually 512 bytes) at the given LBA into `buffer`
    fn read_sector(&self, lba: u64, buffer: &mut [u8; 512], vga_index: &mut isize);

    /// Optional: writes 1 sector from `buffer` to `lba`
    fn write_sector(&self, lba: u64, buffer: &[u8; 512], vga_index: &mut isize);
}

static mut DISK_DATA: [u8; 1024 * 512] = [0u8; 1024 * 512]; // 1024 sectors
const CMD_WRITE_SECTOR: u8 = 0x45; // 0x40 | 0x05 = write with MFM, multi-track

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
    fn read_sector(&self, lba: u64, buffer: &mut [u8; 512], _vga_index: &mut isize) {
        let offset = self.sector_offset(lba);
        let slice = &self.data[offset..offset + 512];
        buffer.copy_from_slice(slice);
    }

    fn write_sector(&self, lba: u64, buffer: &[u8; 512], _vga_index: &mut isize) {
        //let offset = self.sector_offset(lba);
        //let slice = &self.data[offset..offset + 512];
        //slice.copy_from_slice(buffer);
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

//
//
//

const DOR: u16 = 0x3F2;
const MSR: u16 = 0x3F4;
const FIFO: u16 = 0x3F5;

const FDC_DOR: u16 = 0x3F2;
const FDC_MSR: u16 = 0x3F4;
const FDC_DATA: u16 = 0x3F5;

const FDC_CMD_SEEK: u8 = 0x0F;


#[unsafe(link_section = ".dma")]
#[unsafe(no_mangle)]
static mut DMA_BUFFER: [u8; 512] = [0u8; 512];

const DMA_CHANNEL: u8 = 0x02;
const DMA_ADDR_REG: u16 = 0x04; // Channel 2
const DMA_COUNT_REG: u16 = 0x05;
const DMA_PAGE_REG: u16 = 0x81;

const DMA_MASK_REG: u16 = 0x0A;
const DMA_FLIPFLOP_RESET: u16 = 0x0C;
const DMA_MODE_REG: u16 = 0x0B;
const DMA_ADDR_2: u16 = 0x04;
const DMA_COUNT_2: u16 = 0x05;
const DMA_PAGE_2: u16 = 0x81;
const DMA_BUFFER_ADDR: u32 = 0x1000; // Physical address, must be < 64 KiB and page-aligned
const DMA_BUFFER_SIZE: u16 = 512;    // 1 sector

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
    outb(0x0A, DMA_CHANNEL);
}

pub unsafe fn dma_set_read_mode() {
    // Mode: single transfer, address increment, read, channel 2
    outb(0x0B, 0x56);
}

pub unsafe fn dma_set_write_mode() {
    // Mode: single transfer, address increment, write, channel 2
    outb(0x0B, 0x52); // 0101_0010
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

fn fdc_wait_ready() {
    // Wait for RQM and DIO = 1 (ready to send command)
    loop {
        let msr = unsafe { inb(FDC_MSR) };
        if (msr & 0xC0) == 0x80 {
            break;
        }
    }
}

fn fdc_send_byte(byte: u8) {
    fdc_wait_ready();
    unsafe { outb(FDC_DATA, byte); }
}

fn fdc_read_byte() -> u8 {
    fdc_wait_ready();
    unsafe { inb(FDC_DATA) }
}

/// Sense interrupt result (returns st0, cylinder)
fn fdc_sense_interrupt() -> (u8, u8) {
    fdc_send_byte(0x08);
    let st0 = fdc_read_byte();
    let cyl = fdc_read_byte();
    (st0, cyl)
}

/// Dummy IRQ wait — replace with real IRQ handling
fn fdc_wait_irq() {
    // This should block until IRQ6 is received.
    // In a real OS, this should be a semaphore or atomic flag.
    for _ in 0..1000000 {
        // spin-loop / delay (not robust)
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

    pub unsafe fn fdc_wait_irq(vga_index: &mut isize) {
        // Wait until interrupt is fired (simulate or use actual IRQ handling)
        // For now: a naive delay loop or poll status.
        //for _ in 0..10_000_000 {
        loop {
            let status = inb(0x3F4);
            if status & 0x80 != 0 {
                break;
            }
        }

        // Read floppy IRQ status
        Self::send_byte(0x08); // Sense interrupt
                               //
        let st0 = Floppy::read_byte(); // bit 7 = 1 means error
        let st1 = Floppy::read_byte();
        let st2 = Floppy::read_byte();
        let cylinder = Floppy::read_byte();
        let head = Floppy::read_byte();
        let sector = Floppy::read_byte();
        let bytesize = Floppy::read_byte(); // sector size as N where size = 128 << N

        /*crate::vga::write::string(vga_index, b"ST0: ", crate::vga::buffer::Color::Pink);
          crate::vga::write::number(vga_index, st0 as u64);
          crate::vga::write::string(vga_index, b", ST1: ", crate::vga::buffer::Color::Pink);
          crate::vga::write::number(vga_index, st1 as u64);
          crate::vga::write::string(vga_index, b", ST2: ", crate::vga::buffer::Color::Pink);
          crate::vga::write::number(vga_index, st2 as u64);
          crate::vga::write::newline(vga_index);*/
    }

    pub fn read_sector_dma(lba: u64, buffer: &mut [u8; 512], vga_index: &mut isize) {
        let (c, h, s) = lba_to_chs(lba);

        /*crate::vga::write::string(vga_index, b"LBA: ", crate::vga::buffer::Color::Cyan);
          crate::vga::write::number(vga_index, lba);
          crate::vga::write::string(vga_index, b"; CHS: ", crate::vga::buffer::Color::Cyan);
          crate::vga::write::number(vga_index, c as u64);
          crate::vga::write::string(vga_index, b", ", crate::vga::buffer::Color::Cyan);
          crate::vga::write::number(vga_index, h as u64);
          crate::vga::write::string(vga_index, b", ", crate::vga::buffer::Color::Cyan);
          crate::vga::write::number(vga_index, s as u64);
          crate::vga::write::newline(vga_index);*/

        unsafe {
            dma_init(DMA_BUFFER.as_ptr() as u32, 512);
            dma_set_read_mode();

            Self::send_byte(0x46); // READ DATA
            Self::send_byte((h << 2) | 0); // drive 0, head
            Self::send_byte(c);    // cylinder
            Self::send_byte(h);    // head
            Self::send_byte(s);    // sector (1-based)
            Self::send_byte(2);    // 512 = 2^2
            Self::send_byte(18);   // last sector
            Self::send_byte(0x1B); // GAP3
            Self::send_byte(0xFF); // DTL (don't care for 512B)

            Self::fdc_wait_irq(vga_index); // wait for IRQ 6 (must be handled)

            // Copy from DMA buffer to output
            buffer.copy_from_slice(&DMA_BUFFER);

            for byte in DMA_BUFFER.iter_mut() {
                *byte = 0;
            }
        }
    }

    pub fn fdc_dma_setup_write(&self, data: &[u8; 512]) {
        unsafe {
            // Copy data to the known DMA buffer
            core::ptr::copy_nonoverlapping(data.as_ptr(), DMA_BUFFER_ADDR as *mut u8, 512);

            // Mask channel 2
            outb(DMA_MASK_REG, DMA_CHANNEL | 0x04);

            // Reset the flip-flop
            outb(DMA_FLIPFLOP_RESET, 0x00);

            // Send address (low, then high)
            outb(DMA_ADDR_2, (DMA_BUFFER_ADDR & 0xFF) as u8);
            outb(DMA_ADDR_2, ((DMA_BUFFER_ADDR >> 8) & 0xFF) as u8);

            // Send page (high byte of address)
            outb(DMA_PAGE_2, ((DMA_BUFFER_ADDR >> 16) & 0xFF) as u8);

            // Reset flip-flop again
            outb(DMA_FLIPFLOP_RESET, 0x00);

            // Send count (512 - 1 = 511), low then high
            outb(DMA_COUNT_2, (DMA_BUFFER_SIZE - 1) as u8);
            outb(DMA_COUNT_2, ((DMA_BUFFER_SIZE - 1) >> 8) as u8);

            // Set mode:
            // 0x58 = single mode | address increment | write (mem→dev) | channel 2
            outb(DMA_MODE_REG, 0x58);

            // Unmask channel 2
            outb(DMA_MASK_REG, DMA_CHANNEL);
        }
    }

    /// Issues a SEEK command to the FDC.
    /// - `drive`: 0 (A:) or 1 (B:)
    /// - `cylinder`: track number (0..79)
    /// - `head`: 0 or 1
    pub fn fdc_seek(&self, drive: u8, cylinder: u8, head: u8, vga_index: &mut isize) {
        // Select drive in DOR
        let motor_bit = 1 << (4 + drive); // bit 4 = motor for drive 0
        let dor_value = (drive & 0x03) | 0x0C | motor_bit;
        unsafe {
            outb(FDC_DOR, dor_value);
        }

        // Wait until FDC is ready
        fdc_wait_ready();

        crate::vga::write::string(vga_index, b"Skip", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);

        // Send SEEK command
        unsafe {
            fdc_send_byte(FDC_CMD_SEEK);
            fdc_send_byte((head << 2) | (drive & 0x03)); // head & drive combined
            fdc_send_byte(cylinder);
        }

        // Wait for IRQ6 (simplified — use a real IRQ handler ideally)
        fdc_wait_irq();

        unsafe {
            //Self::fdc_wait_irq(vga_index); // wait for IRQ 6 (must be handled)
        }

        crate::vga::write::string(vga_index, b"Kriste", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);


        // Optional: Sense interrupt to verify seek
        //let (_st0, _cyl) = fdc_sense_interrupt();
    }


    pub fn fdc_write_sector(&self, cylinder: u8, head: u8, sector: u8, data: &[u8; 512], vga_index: &mut isize) {
        unsafe {
            //core::arch::asm!("sti");
            dma_init(DMA_BUFFER.as_ptr() as u32, 512);
            //dma_set_write_mode();

        crate::vga::write::string(vga_index, b"Stop", crate::vga::buffer::Color::Yellow);
        crate::vga::write::newline(vga_index);

            self.fdc_seek(1, cylinder, head, vga_index);

            fdc_wait_ready(); // Wait until FDC is ready for commands

            outb(DOR, 0x1C); // Enable motor and controller
            self.fdc_dma_setup_write(data); // Setup DMA for writing

            // Send command packet to FDC
            Self::send_byte(CMD_WRITE_SECTOR);
            Self::send_byte((head << 2) | 0); // Drive 0, head
            Self::send_byte(cylinder);       // Cylinder number
            Self::send_byte(head);           // Head
            Self::send_byte(sector);         // Sector number (starts at 1)
            Self::send_byte(2);              // 512 bytes/sector => 2^2 = 512
            Self::send_byte(18);             // Sectors/track (usually 18)
            Self::send_byte(0x1B);           // GAP3 length (standard = 0x1B)
            Self::send_byte(0xFF);           // Data length (0xFF for default)

            //self.fdc_wait_irq(vga_index);
            Self::fdc_wait_irq(vga_index); // wait for IRQ 6 (must be handled)

            //let status = fdc_read_result();

            //core::arch::asm!("cli");

            // Check status for errors
            //status.iter().all(|&s| s & 0xC0 == 0)
        }
    }



    }

    impl BlockDevice for Floppy {
        fn read_sector(&self, lba: u64, buffer: &mut [u8; 512], vga_index: &mut isize) {
            Self::read_sector_dma(lba, buffer, vga_index);
        }

        fn write_sector(&self, lba: u64, buffer: &[u8; 512], vga_index: &mut isize) {
            let (cylinder, head, sector) = lba_to_chs(lba);

            self.fdc_write_sector(cylinder, head, sector, buffer, vga_index);
        }
    }

