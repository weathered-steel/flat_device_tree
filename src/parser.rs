use core::{fmt, result};
use util::{from_utf8_safe, MiniStream, MiniStreamReadError, display_node};

#[derive(Debug)]
pub enum ParseError {
    InvalidMagic,
    ReadError,
    InvalidTag,
    UnexpectedTag,
    NoRootFound,
}

pub type Result<T> = result::Result<T, ParseError>;

pub struct DeviceTreeParser<'a>
{
    buf: MiniStream<'a>,
    string_offset: usize,

}

#[derive(Debug)]
pub struct DeviceTree {
    header: DeviceTreeHeader,
    root: Node,
}

pub struct Property {
    name: Vec<u8>,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct Node {
    pub name: Vec<u8>,
    pub properties: Vec<Property>,
    pub children: Vec<Node>,
}

impl fmt::Debug for Property {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}: {}",
               from_utf8_safe(self.name.as_slice()),
               from_utf8_safe(self.data.as_slice()))
    }
}

#[derive(Debug, PartialEq)]
enum Tag {
    Property,
    BeginNode,
    EndNode,
    End,
    Magic,
}

impl Tag {
    fn from_u32(val: u32) -> Result<Tag> {
        match val {
            0x01 => Ok(Tag::BeginNode),
            0x02 => Ok(Tag::EndNode),
            0x03 => Ok(Tag::Property),
            0x09 => Ok(Tag::End),
            0xd00dfeed => Ok(Tag::Magic),
            _ => Err(ParseError::InvalidTag),
        }
    }
}

impl<'a> DeviceTreeParser<'a> {
    pub fn new(buf: &'a [u8]) -> DeviceTreeParser<'a> {
        DeviceTreeParser{
            buf: MiniStream::new(buf),
            string_offset: 0,
        }
    }

    pub fn pos(&self) -> usize {
        self.buf.pos()
    }

    fn tag(&mut self) -> Result<Tag> {
        Ok(try!(Tag::from_u32(try!(self.buf.read_u32_le()))))
    }

    fn peek_tag(&self) -> Result<Tag> {
        Ok(try!(Tag::from_u32(try!(self.buf.peek_u32_le()))))
    }

    fn accept_tag(&mut self, tag: Tag) -> Result<bool> {
        let t = try!(self.peek_tag());

        if t == tag {
            try!(self.tag());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect_tag(&mut self, tag: Tag) -> Result<()> {
        if ! try!(self.accept_tag(tag)) {
            Err(ParseError::UnexpectedTag)
        } else {
            Ok(())
        }
    }

    fn block_string0(&mut self) -> Result<&[u8]> {
        let start = self.buf.pos();
        let mut num_blocks = 0;
        let offset;

        'search: loop {
            let block = try!(self.buf.read_bytes(4));
            num_blocks += 1;

            for i in 0..4 {
                if block[i] == 0 {
                    offset = 4-i;
                    break 'search;
                }
            }
        }

        try!(self.buf.seek(start));
        let data = try!(self.buf.read_bytes(num_blocks * 4));
        Ok(&data[..data.len()-offset])
    }

    fn node(&mut self) -> Result<Option<Node>> {
        if try!(self.accept_tag(Tag::BeginNode)) {
            let name = try!(self.block_string0()).to_owned();
            let mut rs = Node{
                properties: Vec::new(),
                children: Vec::new(),
                name: name,
            };

            while try!(self.accept_tag(Tag::Property)) {
                let prop_data_len = try!(self.buf.read_u32_le());

                let prop_name_offset = try!(self.buf.read_u32_le());

                let mut prop_data = Vec::new();
                prop_data.extend(
                    try!(self.buf.read_bytes(prop_data_len as usize))
                );

                // re-align to 4 byte blocks
                try!(self.buf.align());

                let prop_val_name = try!(
                    self.far_string0(prop_name_offset as usize)
                );

                let prop = Property{
                    name: prop_val_name,
                    data: prop_data,
                };
                rs.properties.push(prop);
            }

            // after properties, read child nodes
            loop {
                if let Some(child) = try!(self.node()) {
                    rs.children.push(child)
                } else {
                    break
                }
            }

            // proper end node needed
            try!(self.expect_tag(Tag::EndNode));

            Ok(Some(rs))
        } else {
            Ok(None)
        }
    }

    fn far_string0(&mut self, offset: usize) -> Result<Vec<u8>> {
        let pos = self.pos();

        try!(self.buf.seek(self.string_offset + offset));
        let buf = try!(self.buf.read_string0()).to_owned();

        try!(self.buf.seek(pos));

        Ok(buf)
    }

    pub fn parse(&mut self) -> Result<DeviceTree> {
        // // first, read the header
        if try!(self.tag()) != Tag::Magic {
            return Err(ParseError::InvalidMagic);
        }

        let totalsize = try!(self.buf.read_u32_le());
        let off_dt_struct = try!(self.buf.read_u32_le());
        let off_dt_strings = try!(self.buf.read_u32_le());
        self.string_offset = off_dt_strings as usize;
        let off_mem_rsvmap = try!(self.buf.read_u32_le());
        let version = try!(self.buf.read_u32_le());
        let last_comp_version = try!(self.buf.read_u32_le());

        let mut boot_cpuid_phys = 0;
        if version > 2 {
            boot_cpuid_phys = try!(self.buf.read_u32_le());
        }

        // version 3 fields
        let mut size_dt_strings = 0;
        if version > 3 {
            size_dt_strings = try!(self.buf.read_u32_le())
        }

        // version 17 fields
        let mut size_dt_struct = 0;
        if version > 17 {
            size_dt_struct = try!(self.buf.read_u32_le());
        }

        let header = DeviceTreeHeader{
            totalsize: totalsize,
            off_dt_struct: off_dt_struct,
            off_dt_strings: off_dt_strings,
            off_mem_rsvmap: off_mem_rsvmap,
            version: version,
            last_comp_version: last_comp_version,

            // version 2 fields
            boot_cpuid_phys: boot_cpuid_phys,

            // version 3 fields
            size_dt_strings: size_dt_strings,

            // version 17 fields
            size_dt_struct: size_dt_struct,
        };

        // read Node first
        try!(self.buf.seek(off_dt_struct as usize));

        match try!(self.node()) {
            None => Err(ParseError::NoRootFound),
            Some(root) => Ok(DeviceTree{
                header: header,
                root: root,
            })
        }
    }
}

#[derive(Debug)]
struct DeviceTreeHeader {
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,

    // version 2 fields
    boot_cpuid_phys: u32,

    // version 3 fields
    size_dt_strings: u32,

    // version 17 fields
    size_dt_struct: u32,
}

impl fmt::Display for DeviceTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "device tree (version {}, compat {}, boot cpu {}), {} bytes\n\
               {}"
               ,
               self.header.version,
               self.header.last_comp_version,
               self.header.boot_cpuid_phys,
               self.header.totalsize,
               self.root)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        display_node(self, f, 0)
    }
}

impl From<MiniStreamReadError> for ParseError {
    fn from(_: MiniStreamReadError) -> ParseError {
        ParseError::ReadError
    }
}
