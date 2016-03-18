use core::result;

#[derive(Debug)]
pub enum MiniStreamReadError {
    ReadPastEnd
}

#[derive(Debug)]
pub struct MiniStream<'a>{
    buf: &'a [u8],
    pos: usize,
}

impl<'a> MiniStream<'a> {
    pub fn new(buf: &'a [u8]) -> MiniStream<'a> {
        MiniStream{
            buf: buf,
            pos: 0,
        }
    }

    pub fn read_byte(&mut self) -> result::Result<u8, MiniStreamReadError> {
        if self.pos+1 < self.buf.len() {
            let byte = self.buf[self.pos];
            self.pos += 1;
            Ok(byte)
        } else {
            Err(MiniStreamReadError::ReadPastEnd)
        }
    }

    pub fn read_bytes(&mut self, len: usize)
    -> result::Result<&[u8], MiniStreamReadError> {
        if self.pos + len < self.buf.len() {
            let block = &self.buf[self.pos..self.pos+len];
            self.pos += len;
            Ok(block)
        } else {
            Err(MiniStreamReadError::ReadPastEnd)
        }
    }

    pub fn read_u32_le(&mut self) -> result::Result<u32, MiniStreamReadError> {
        if self.pos + 4 < self.buf.len() {
            let val: u32 = unsafe {
                *(self.buf[self.pos..(self.pos+4)].as_ptr() as *const u32)
            };
            self.pos += 4;

            // FIXME: determine endianness and properly convert
            Ok((val >> 24) & 0xff
              |(val >> 8) & 0xff00
              |(val << 8) & 0xff0000
              |(val << 24)  & 0xff000000)
        } else {
            Err(MiniStreamReadError::ReadPastEnd)
        }
    }

    pub fn seek(&mut self, position: usize)
    -> result::Result<(), MiniStreamReadError> {
        if position >= self.buf.len() {
            Err(MiniStreamReadError::ReadPastEnd)
        } else {
            self.pos = position;
            Ok(())
        }
    }
}
