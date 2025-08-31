pub trait BlockDevice {
    /// Reads 1 sector (usually 512 bytes) at the given LBA into `buffer`
    fn read_sector(&self, lba: u64, buffer: &mut [u8]);

    /// Writes 1 sector from `buffer` to `lba`
    fn write_sector(&self, lba: u64, buffer: &[u8; 512]);
}

static mut DISK_DATA: [u8; 1024 * 512] = [0u8; 1024 * 512]; // 1024 sectors
const CMD_WRITE_SECTOR: u8 = 0x45; // 0x40 | 0x05 = write with MFM, multi-track

pub struct MemDisk {
    pub data: &'static mut [u8], // Must be sector-aligned
}

//
//  MEMDISK
//

impl MemDisk {
    pub fn new(data: &'static mut [u8]) -> Self {
        Self { data }
    }

    fn sector_offset(&self, lba: u64) -> usize {
        (lba as usize) * 512
    }
}

impl BlockDevice for MemDisk {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) {
        let offset = self.sector_offset(lba);
        let slice = &self.data[offset..offset + 512];
        buffer.copy_from_slice(slice);
    }

    fn write_sector(&self, _lba: u64, _buffer: &[u8; 512]) {
        //let offset = self.sector_offset(lba);
        //let slice = &self.data[offset..offset + 512];
        //slice.copy_from_slice(buffer);
    }
}

//
//  FLOPPY
//

pub struct Floppy;

impl BlockDevice for Floppy {
    fn read_sector(&self, lba: u64, buffer: &mut [u8]) {
        let (c, h, s) = self.lba_to_chs(lba);

        #[expect(static_mut_refs)]
        unsafe {
            Self::init();
            self.set_read_mode();

            self.send_byte(0x46);          // Read data
            self.send_byte(h << 2);  // drive 0, head
            self.send_byte(c);             // Cylinder
            self.send_byte(h);             // Head
            self.send_byte(s);             // Sector (1-based)
            self.send_byte(2);             // 512 = 2^2
            self.send_byte(18);            // Last sector
            self.send_byte(0x1B);          // GAP3
            self.send_byte(0xFF);          // DTL (don't care for 512B)

            self.wait_for_irq();    // Wait for IRQ 6
            
            /*for i in 0..10 {
                debugn!(dma[i]);
            }
            debugln!("");*/

            // Copy from DMA buffer to output

            core::ptr::copy_nonoverlapping(DMA.as_ptr(), buffer.as_mut_ptr(), 512);

            for byte in DMA.iter_mut() {
                *byte = 0;
            }
        }
    }

    fn write_sector(&self, lba: u64, buffer: &[u8; 512]) {
        //debugln!("Floppy write_sector()");

        let (cylinder, head, sector) = self.lba_to_chs(lba);
        self.write_sector(cylinder, head, sector, buffer);
    }
}

//
//
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

//
//  DMA
//  https://wiki.osdev.org/ISA_DMA
//

/*extern "C" {
    static mut dma: [u8; 512];
}*/

#[unsafe(link_section = ".dma")]
pub static mut DMA: [u8; 512] = [0; 512];

//#[unsafe(link_section = ".dma")]
//#[unsafe(no_mangle)]
//static mut DMA_BUFFER: [u8; 512] = [0u8; 512];

const DOR: u16 = 0x3F2;
const MSR: u16 = 0x3F4;
const FIFO: u16 = 0x3F5;

const FDC_DOR: u16 = 0x3F2;
const FDC_MSR: u16 = 0x3F4;
const FDC_DATA: u16 = 0x3F5;

const FDC_CMD_SEEK: u8 = 0x0F;

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

//
//
//

impl Floppy {
    pub fn init() -> Self {
        unsafe {
            outb(DOR, 0x1C); // Motor on, DMA/IRQ enabled

            let count = 512;

            let addr = &raw const DMA as u32;

            //debugn!(&dma as *const _ as usize);
            //debugln!("");

            let page = ((addr >> 16) & 0xFF) as u8;
            let offset = addr as u16;

            // Mask channel 2+0
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
        Self
    }

    fn set_read_mode(&self) {
        unsafe {
            // Mode: single transfer, address increment, read, channel 2
            outb(0x0B, 0x56);
        }
    }

    fn set_write_mode(&self) {
        unsafe {
            // Mode: single transfer, address increment, write, channel 2
            outb(0x0B, 0x52); // 0101_0010
        }
    }

    fn lba_to_chs(&self, lba: u64) -> (u8, u8, u8) {
        let sectors_per_track = 18;
        let heads = 2;

        let cylinder = (lba / (sectors_per_track * heads)) as u8;
        let temp = lba % (sectors_per_track * heads);
        let head = (temp / sectors_per_track) as u8;
        let sector = (temp % sectors_per_track + 1) as u8; // 1-based

        (cylinder, head, sector)
    }

    fn send_byte(&self, byte: u8) {
        unsafe {
            for _ in 0..100000 {
                if inb(0x3F4) & 0x80 != 0 {
                    break;
                }
            }
            outb(0x3F5, byte);
        }
    }

    fn read_byte(&self) -> u8 {
        unsafe {
            for _ in 0..100000 {
                if inb(0x3F4) & 0x80 != 0 {
                    return inb(0x3F5);
                }
            }
            0
        }
    }

    fn wait_ready(&self) {
        // Wait for RQM and DIO = 1 (ready to send command)
        loop {
            let msr = unsafe { inb(FDC_MSR) };
            if (msr & 0xC0) == 0x80 {
                break;
            }
        }
    }

    fn wait_for_irq(&self) {
        unsafe {
            // Wait until interrupt is fired
            loop {
                let status = inb(0x3F4);
                if status & 0x80 != 0 {
                    break;
                }
            }

            // Read floppy IRQ status
            self.send_byte(0x08); // Sense interrupt

            let _st0 = self.read_byte(); // bit 7 = 1 means error
            let _st1 = self.read_byte();
            let _st2 = self.read_byte();
            let _cylinder = self.read_byte();
            let _head = self.read_byte();
            let _sector = self.read_byte();
            let _bytesize = self.read_byte(); // Sector size as N where size = 128 << N

            // TODO: Dump controller status, check for errors
        }
    }

    fn setup_write(&self, data: &[u8; 512]) {
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
    fn seek(&self, drive: u8, cylinder: u8, head: u8) {
        // Select drive in DOR
        let motor_bit = 1 << (4 + drive); // Bit 4 = motor for drive 0
        let dor_value = (drive & 0x03) | 0x0C | motor_bit;
        unsafe {
            outb(FDC_DOR, dor_value);
        }

        // Wait until FDC is ready
        self.wait_ready();

        // Send SEEK command
        self.send_byte(FDC_CMD_SEEK);
        self.send_byte((head << 2) | (drive & 0x03)); // Head & drive combined
        self.send_byte(cylinder);

        // Wait for IRQ6
        self.wait_for_irq();

        // TODO: Verify the seek result (sense the interrupt)
    }

    fn write_sector(&self, cylinder: u8, head: u8, sector: u8, data: &[u8; 512]) {
        unsafe {
            //core::arch::asm!("sti");
            //self.set_write_mode();

            self.seek(1, cylinder, head);

            self.wait_ready();

            outb(DOR, 0x1C);   // Enable motor and controller
            self.setup_write(data); // Setup DMA for writing

            // Send command packet to FDC
            self.send_byte(CMD_WRITE_SECTOR);
            self.send_byte(head << 2);         // Drive 0, head
            self.send_byte(cylinder);          // Cylinder number
            self.send_byte(head);              // Head
            self.send_byte(sector);            // Sector number (starts at 1)
            self.send_byte(2);                 // 512 bytes/sector => 2^2 = 512
            self.send_byte(18);                // Sectors/track (usually 18)
            self.send_byte(0x1B);              // GAP3 length (standard = 0x1B)
            self.send_byte(0xFF);              // Data length (0xFF for default)

            self.wait_for_irq(); // wait for IRQ 6 (must be handled)

            //let status = fdc_read_result();

            //core::arch::asm!("cli");

            // TODO: Check status for errors
            //status.iter().all(|&s| s & 0xC0 == 0)
        }
    }
}

