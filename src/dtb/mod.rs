pub mod parser;
pub mod stream;

pub use parser::*;
pub use stream::*;

use core::ffi::CStr;

#[derive(Clone, Copy, Debug)]
pub enum DtbError {
    InvalidMagic(u32),
    ByteStream,
    InvalidStructTok(u32),
    InvalidHeader,
    ExpectedOneNode,
    ExpectedProperty,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DtbHeader {
    /// Magic number identifying this as a DTB file (must be 0xd00dfeed).
    pub magic: u32,
    /// Total size of the entire DTB file in bytes.
    pub totalsize: u32,
    /// Byte offset from start of file to the structure block.
    pub off_dt_struct: u32,
    /// Byte offset from start of file to the strings block.
    pub off_dt_strings: u32,
    /// Byte offset from start of file to the memory reservation block.
    pub off_mem_rsvmap: u32,
    /// DTB format version number (typically 17 for modern files).
    pub version: u32,
    /// Last DTB version that this file is compatible with.
    pub last_comp_version: u32,
    /// Physical CPU ID of the boot processor.
    pub boot_cpuid_phys: u32,
    /// Size of the strings block in bytes.
    pub size_dt_strings: u32,
    /// Size of the structure block in bytes.
    pub size_dt_struct: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Reserved {
    pub address: u64,
    pub size: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Property<'a> {
    pub name: &'a CStr,
    pub data: ByteStream<'a>,
}

impl<'a> Property<'a> {
    pub fn new(name: &'a CStr, data: ByteStream<'a>) -> Self {
        Self { name, data }
    }
}

impl DtbHeader {
    pub const MAGIC: u32 = 0xd00d_feed;
}

pub struct Dtb<'a>(&'a [u8]);

impl<'a> Dtb<'a> {
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self(slice)
    }

    pub unsafe fn from_ptr(dtb: *const u8) -> Result<Self, DtbError> {
        unsafe {
            let magic = u32::from_be_bytes(dtb.cast::<u32>().read().to_ne_bytes());
            if magic != DtbHeader::MAGIC {
                return Err(DtbError::InvalidMagic(magic));
            }
            let total_size = u32::from_be_bytes(dtb.add(4).cast::<u32>().read().to_ne_bytes());

            Ok(Dtb(core::slice::from_raw_parts(dtb, total_size as usize)))
        }
    }

    pub fn stream(&self, at: usize) -> Result<ByteStream<'a>, DtbError> {
        match self.0.get(at..) {
            Some(buf) => Ok(ByteStream::new(buf, at)),
            None => Err(DtbError::InvalidHeader),
        }
    }

    pub fn limited_stream(&self, at: usize, limit: usize) -> Result<ByteStream<'a>, DtbError> {
        match self.0.get(at..) {
            Some(buf) => Ok(ByteStream::new(buf, at).limit(limit)?),
            None => Err(DtbError::InvalidHeader),
        }
    }

    pub fn reserved_parser(&self) -> Result<DtbReserveParser<'a>, DtbError> {
        Ok(DtbReserveParser::new(
            self.stream(self.header()?.off_mem_rsvmap as usize)?,
        ))
    }

    pub fn strings(&self) -> Result<DtbStrings<'a>, DtbError> {
        let header = self.header()?;
        Ok(DtbStrings::new(
            self.limited_stream(
                header.off_dt_strings as usize,
                header.size_dt_strings as usize,
            )?
            .buf(),
        ))
    }

    pub fn struct_parser(&self) -> Result<DtbStructParser<'a>, DtbError> {
        Ok(DtbStructParser::new(self.struct_stream()?, self.strings()?))
    }

    pub fn struct_stream(&self) -> Result<ByteStream<'a>, DtbError> {
        let header = self.header()?;
        self.limited_stream(
            header.off_dt_struct as usize,
            header.size_dt_struct as usize,
        )
    }

    pub fn header(&self) -> Result<DtbHeader, DtbError> {
        let mut stream = self.stream(0)?;

        let magic = stream.u32()?;
        if magic != DtbHeader::MAGIC {
            return Err(DtbError::InvalidMagic(magic));
        }
        let totalsize = stream.u32()?;
        if (totalsize as usize) < self.0.len() {
            return Err(DtbError::InvalidHeader);
        }
        Ok(DtbHeader {
            magic,
            totalsize,
            off_dt_struct: stream.u32()?,
            off_dt_strings: stream.u32()?,
            off_mem_rsvmap: stream.u32()?,
            version: stream.u32()?,
            last_comp_version: stream.u32()?,
            boot_cpuid_phys: stream.u32()?,
            size_dt_strings: stream.u32()?,
            size_dt_struct: stream.u32()?,
        })
    }

    pub fn find_compatable_nodes<'b, 'c>(
        &self,
        compatable: &'b [u8],
    ) -> Result<CompatableNodeIter<'c>, DtbError>
    where
        'a: 'c,
        'b: 'c,
    {
        Ok(CompatableNodeIter::new(self.struct_parser()?, compatable))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DtbNode<'a> {
    parser: DtbStructParser<'a>,
}

impl<'a> DtbNode<'a> {
    pub fn new(parser: DtbStructParser<'a>) -> Self {
        Self { parser }
    }
    
    pub fn name(&self) -> Result<&'a CStr, DtbError> {
        let mut stream = self.parser.stream();
        stream.u32()?;
        stream.cstr()
    }

    pub fn properties(&self) -> Result<DtbNodePropertyParser<'a>, DtbError> {
        let mut stream = self.parser.stream();
        stream.u32()?;
        stream.cstr()?;
        stream.align(4)?;
        Ok(DtbNodePropertyParser::new(DtbStructParser::new(
            stream,
            self.parser.strings(),
        )))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CompatableNodeIter<'a> {
    parser: DtbStructParser<'a>,
    compatable: &'a [u8],
}

impl<'a> CompatableNodeIter<'a> {
    pub fn new(parser: DtbStructParser<'a>, compatable: &'a [u8]) -> Self {
        Self { parser, compatable }
    }

    pub fn expect_one(mut self) -> Result<DtbNode<'a>, DtbError>{
        let node = self.next()?.ok_or(DtbError::ExpectedOneNode)?;

        if self.next()?.is_some(){
            return Err(DtbError::ExpectedOneNode)
        }
        Ok(node)
    }

    pub fn next(&mut self) -> Result<Option<DtbNode<'a>>, DtbError> {
        let mut entry = None;
        loop {
            let before = self.parser;
            let Some(tok) = self.parser.next()? else {
                return Ok(None);
            };
            match tok {
                Tok::BeginNode(_) => entry = Some(before),
                Tok::Prop(Property { name, mut data }) if name.to_bytes() == b"compatible" => {
                    while let Ok(str) = data.cstr() {
                        if str.to_bytes() == self.compatable {
                            return Ok(Some(DtbNode::new(
                                entry.ok_or(DtbError::ByteStream)?,
                            )));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
