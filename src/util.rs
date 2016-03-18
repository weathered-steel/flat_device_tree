use core::{fmt, result, str};

use parser::Node;

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

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn align(&mut self) -> result::Result<(), MiniStreamReadError> {
        let m = self.pos % 4;
        if m != 0 {
            let step = 4 - m;
            if step + self.pos >= self.buf.len() {
                return Err(MiniStreamReadError::ReadPastEnd)
            }

            self.pos += step
        }
        Ok(())
    }

    pub fn read_string0(&mut self)
    -> result::Result<&[u8], MiniStreamReadError> {
        let start = self.pos;

        loop {
            if self.buf[self.pos] == 0 {
                self.pos += 1;
                break
            }
            self.pos += 1;

            if ! self.pos < self.buf.len() {
                return Err(MiniStreamReadError::ReadPastEnd)
            }
        }

        Ok(&self.buf[start..self.pos])
    }

    pub fn read_bytes(&mut self, len: usize)
    -> result::Result<&[u8], MiniStreamReadError> {
        if self.pos + len < self.buf.len() {
            let val = &self.buf[self.pos..self.pos+len];
            self.pos += len;
            Ok(val)
        } else {
            Err(MiniStreamReadError::ReadPastEnd)
        }
    }

    pub fn peek_bytes(&self, len: usize)
    -> result::Result<&[u8], MiniStreamReadError> {
        if self.pos + len < self.buf.len() {
            Ok(&self.buf[self.pos..self.pos+len])
        } else {
            Err(MiniStreamReadError::ReadPastEnd)
        }
    }

    pub fn peek_u32_le(&self) -> result::Result<u32, MiniStreamReadError> {
        if self.pos + 4 < self.buf.len() {
            let val: u32 = unsafe {
                *(self.buf[self.pos..(self.pos+4)].as_ptr() as *const u32)
            };

            // FIXME: determine endianness and properly convert
            Ok((val >> 24) & 0xff
              |(val >> 8) & 0xff00
              |(val << 8) & 0xff0000
              |(val << 24)  & 0xff000000)
        } else {
            Err(MiniStreamReadError::ReadPastEnd)
        }
    }

    pub fn read_u32_le(&mut self) -> result::Result<u32, MiniStreamReadError> {
        let val = try!(self.peek_u32_le());
        self.pos += 4;
        Ok(val)
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

pub fn from_utf8_safe(v: &[u8]) -> &str {
    match str::from_utf8(v) {
        Ok(s) => s,
        Err(_) => "utf8!invalid"
    }
}

pub fn display_node(node: &Node, f: &mut fmt::Formatter, indent: u32)
-> fmt::Result  {
    let mut indt = String::new();
    for _ in 0..indent {
        indt = indt + "   ";
    }

    if node.name.len() == 0 {
        try!(write!(f, "{}|- /\n", indt));
    } else {
        try!(write!(f, "{}|- {:?}\n", indt, from_utf8_safe(&node.name)));
    }

    for prop in node.properties.iter() {
        try!(write!(f, "{}   {:?}\n", indt, prop));
    }

    for child in node.children.iter() {
        try!(display_node(child, f, indent + 1))
    }

    Ok(())
}
