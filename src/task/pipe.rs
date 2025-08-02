const MAX_BUFFER_SIZE: usize = 14096;

#[derive(Copy,Clone)]
#[repr(C, packed)]
pub struct Pipe {
    buffer: [u8; MAX_BUFFER_SIZE],
    id: u64,
    pub read_pos: usize,
    pub write_pos: usize,
}

impl Pipe {
    pub fn new(id: u64) -> Self {
        let pipe = Pipe{
            buffer: [0u8; MAX_BUFFER_SIZE],
            id,
            read_pos: 0,
            write_pos: 0,
        };
        pipe
    }

    pub fn read(&mut self) -> u8 {
        if self.read_pos == self.write_pos {
            return 0x00;
        }

        let output = self.buffer[self.read_pos];

        self.read_pos += 1;
        self.read_pos %= MAX_BUFFER_SIZE;

        /*if self.read_pos < self.write_pos {
            if let Some(buf) = self.buffer.get(self.read_pos..self.write_pos) {
                output.copy_from_slice(buf);
            }
        }*/

        output
    }

    pub fn write(&mut self, ch: u8) {
        self.buffer[self.write_pos] = ch;

        self.write_pos += 1;
        self.write_pos %= MAX_BUFFER_SIZE;
    }
}
