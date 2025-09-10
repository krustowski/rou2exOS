//generic buffer reusables

const BUFFER_SIZE: usize = 1024;


pub struct Buffer {
    pub buf: [u8; 1024],
    pub pos: usize,
}
const MAX_MSG_LEN: usize = 60;

/*pub fn placeholder() {
	//key created
	if let Some(mut key) = sysprint::SysBuffer.trylock() {
		key.pos;



	}




}; */
	


impl Buffer {
    //get buffer instance
    pub const fn new() -> Self {
        Self {
            buf: [0u8; BUFFER_SIZE], //array of u8 of 1024
            pos: 0,
        }
    }

	pub fn format(&mut self, message: &'static str) {
		let len = message.len();
		
		//move -> then add [1234]
		

}

    pub fn append(&mut self, s: &[u8]) {
        // input length or offset
		//s.len gives length of whats to be written, min compares minimum of comparison of 
		//selfs buf len - position
		//make self buf len maybe a const?
        let len = s.len().min(self.buf.len() - self.pos);
		//get mut returns mutable reference of self pos + len
        if let Some(buf) = self.buf.get_mut(self.pos..self.pos + len) {
            if let Some(slice) = s.get(..len) {
                // Copy the slice into buffer at offset of self.pos
                buf.copy_from_slice(slice);
                self.pos += len;
            }
        }
    }

    /// Puts the contents of buf into the printb! macro.
    pub fn flush(&self) {
        if let Some(buf) = self.buf.get(..self.pos) {
            printb!(buf);
        }
    }
}