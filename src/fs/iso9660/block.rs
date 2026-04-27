/// ATAPI PIO driver — secondary IDE master (0x170), hardcoded for QEMU `-cdrom`.
/// Sector size: 2048 bytes (ISO9660 logical block).

const BASE: u16 = 0x170;
const DATA:            u16 = BASE;
const FEATURES:        u16 = BASE + 1;
const BYTE_COUNT_LOW:  u16 = BASE + 4;
const BYTE_COUNT_HIGH: u16 = BASE + 5;
const DRIVE_SELECT:    u16 = BASE + 6;
const COMMAND_STATUS:  u16 = BASE + 7;
const ALT_STATUS:      u16 = 0x376;

pub const BLOCK_SIZE: usize = 2048;

pub struct Atapi;

impl Atapi {
    pub const fn new() -> Self { Self }

    /// Read one 2048-byte ISO9660 block at `lba` into `buf`.
    /// Returns false if the device is absent, times out, or signals an error.
    pub fn read_block(&self, lba: u32, buf: &mut [u8; BLOCK_SIZE]) -> bool {
        unsafe { self.read_inner(lba, buf) }
    }

    unsafe fn read_inner(&self, lba: u32, buf: &mut [u8; BLOCK_SIZE]) -> bool {
        outb(DRIVE_SELECT, 0xA0);   // select secondary master
        io_delay();                  // give drive time to see the select

        outb(FEATURES, 0x00);                // PIO, no DMA
        outb(BYTE_COUNT_LOW,  0x00);         // max transfer = 2048 = 0x0800
        outb(BYTE_COUNT_HIGH, 0x08);
        outb(COMMAND_STATUS, 0xA0);          // ATA PACKET command

        if !wait_drq() { return false; }

        // 12-byte READ(10) ATAPI packet, written as 6 × u16 (little-endian).
        let b2 = (lba >> 24) as u16 & 0xFF;
        let b3 = (lba >> 16) as u16 & 0xFF;
        let b4 = (lba >>  8) as u16 & 0xFF;
        let b5 =  lba        as u16 & 0xFF;
        outw(DATA, 0x0028);              // bytes [0]=READ(10), [1]=0
        outw(DATA, b2 | (b3 << 8));      // bytes [2..3] = LBA[31:16]
        outw(DATA, b4 | (b5 << 8));      // bytes [4..5] = LBA[15:0]
        outw(DATA, 0x0000);              // bytes [6..7] = 0
        outw(DATA, 0x0001);              // bytes [8]=xfer_count=1, [9]=0
        outw(DATA, 0x0000);              // bytes [10..11] = 0

        if !wait_drq() { return false; }

        // Read 1024 × u16 = 2048 bytes.
        for i in 0..1024_usize {
            let w = inw(DATA);
            buf[i * 2]     = w as u8;
            buf[i * 2 + 1] = (w >> 8) as u8;
        }

        wait_not_busy();
        true
    }
}

#[inline(always)]
unsafe fn outb(port: u16, v: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") v, options(nomem, nostack));
}
#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    v
}
#[inline(always)]
unsafe fn outw(port: u16, v: u16) {
    core::arch::asm!("out dx, ax", in("dx") port, in("ax") v, options(nomem, nostack));
}
#[inline(always)]
unsafe fn inw(port: u16) -> u16 {
    let v: u16;
    core::arch::asm!("in ax, dx", out("ax") v, in("dx") port, options(nomem, nostack));
    v
}

/// Reads alt-status 4 times — ~400 ns delay that lets the drive assert BSY.
#[inline(always)]
unsafe fn io_delay() {
    for _ in 0..4usize { let _ = inb(ALT_STATUS); }
}

/// Wait until BSY=0 and DRQ=1.  Returns false on error or timeout.
unsafe fn wait_drq() -> bool {
    for _ in 0..0x80000u32 {
        let s = inb(COMMAND_STATUS);
        if s & 0x80 == 0 {           // BSY clear
            if s & 0x08 != 0 { return true; }   // DRQ set
            if s & 0x01 != 0 { return false; }  // ERR
        }
    }
    false
}

/// Spin until BSY clears (after reading the last data word).
unsafe fn wait_not_busy() {
    for _ in 0..0x80000u32 {
        if inb(COMMAND_STATUS) & 0x80 == 0 { return; }
    }
}
