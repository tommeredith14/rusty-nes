pub fn read_word_from_buffer(buf: &[u8], offset: usize) -> u16 {
    (buf[offset] as u16) | ((buf[offset + 1] as u16) << 8)
}

// Consider adding these?
// pub trait Readable {
//     fn read_byte(&self, address: u16) -> u8;
//     fn read_word(&self, address: u16) -> u16;
// }

// pub trait MutReadable {
//     fn read_byte(&mut self, address: u16) -> u8;
//     fn read_word(&mut self, address: u16) -> u16;
// }

// pub trait Writable {
//     fn write_byte(&mut self, address: u16, val: u8);
// }
