pub mod dump;
pub mod iter;
pub mod stream;
pub mod strings;

pub use iter::*;
pub use stream::*;
pub use strings::*;

pub use dump::dump;

use core::ffi::CStr;

#[derive(Clone, Copy, Debug)]
pub enum DtbError {
    InvalidMagic(u32),
    InvalidStructTok(u32),
    InvalidHeader,
    InvalidStrings,
    InvalidStruct,
    InvalidReserved,
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

impl DtbHeader {
    pub const MAGIC: u32 = 0xd00d_feed;
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
    pub const fn new(name: &'a CStr, data: ByteStream<'a>) -> Self {
        Self { name, data }
    }
}

pub struct Dtb<'a> {
    header: DtbHeader,
    reserved: ByteStream<'a>,
    structure: ByteStream<'a>,
    strings: DtbStrings<'a>,
}

impl<'a> Dtb<'a> {
    pub fn from_slice(slice: &'a [u8]) -> Result<Self, DtbError> {
        let header = parse_header(slice)?;

        fn parse_header(slice: &[u8]) -> Result<DtbHeader, DtbError> {
            let mut stream = ByteStream::new(slice, 0);
            let magic = stream.u32().ok_or(DtbError::InvalidHeader)?;
            if magic != DtbHeader::MAGIC {
                return Err(DtbError::InvalidMagic(magic));
            }
            let totalsize = stream.u32().ok_or(DtbError::InvalidHeader)?;
            if (totalsize as usize) < slice.len() {
                return Err(DtbError::InvalidHeader);
            }
            Ok(DtbHeader {
                magic,
                totalsize,
                off_dt_struct: stream.u32().ok_or(DtbError::InvalidHeader)?,
                off_dt_strings: stream.u32().ok_or(DtbError::InvalidHeader)?,
                off_mem_rsvmap: stream.u32().ok_or(DtbError::InvalidHeader)?,
                version: stream.u32().ok_or(DtbError::InvalidHeader)?,
                last_comp_version: stream.u32().ok_or(DtbError::InvalidHeader)?,
                boot_cpuid_phys: stream.u32().ok_or(DtbError::InvalidHeader)?,
                size_dt_strings: stream.u32().ok_or(DtbError::InvalidHeader)?,
                size_dt_struct: stream.u32().ok_or(DtbError::InvalidHeader)?,
            })
        }

        let strings_start: usize = header
            .off_dt_strings
            .try_into()
            .map_err(|_| DtbError::InvalidHeader)?;
        let strings_len: usize = header
            .size_dt_strings
            .try_into()
            .map_err(|_| DtbError::InvalidHeader)?;
        let strings = DtbStrings::new(
            match slice.get(strings_start..strings_start + strings_len) {
                Some(slice) => slice,
                None => return Err(DtbError::InvalidHeader),
            },
        );

        let struct_start: usize = header
            .off_dt_struct
            .try_into()
            .map_err(|_| DtbError::InvalidHeader)?;
        let struct_len: usize = header
            .size_dt_struct
            .try_into()
            .map_err(|_| DtbError::InvalidHeader)?;
        let structure = match slice.get(struct_start..struct_start + struct_len) {
            Some(slice) => ByteStream::new(slice, struct_start),
            None => return Err(DtbError::InvalidHeader),
        };

        let reserved_start = header
            .off_mem_rsvmap
            .try_into()
            .map_err(|_| DtbError::InvalidHeader)?;
        let reserved = match slice.get(reserved_start..) {
            Some(slice) => ByteStream::new(slice, reserved_start),
            None => return Err(DtbError::InvalidHeader),
        };

        // verify that structure is well formed and 'aligned'

        {
            if structure.offset() % 4 != 0 {
                return Err(DtbError::InvalidHeader);
            }

            let mut stream = structure;
            let mut level = 0;
            let mut past_properties = false;
            loop {
                match stream.u32().ok_or(DtbError::InvalidStruct)? {
                    // begin node
                    0x01 => {
                        stream.cstr().ok_or(DtbError::InvalidStruct)?;
                        stream.align(4);
                        level += 1;
                        past_properties = true;
                    }
                    // prop
                    0x03 if !past_properties => return Err(DtbError::InvalidStruct),
                    0x03 => {
                        let length = stream
                            .u32()
                            .ok_or(DtbError::InvalidStruct)?
                            .try_into()
                            .map_err(|_| DtbError::InvalidStruct)?;
                        let nameoff = stream
                            .u32()
                            .ok_or(DtbError::InvalidStruct)?
                            .try_into()
                            .map_err(|_| DtbError::InvalidStruct)?;
                        let _ = stream.bytes(length).ok_or(DtbError::InvalidStruct)?;
                        stream.align(4);
                        strings.get(nameoff).ok_or(DtbError::InvalidStrings)?;
                    }
                    // end node
                    0x02 => {
                        past_properties = true;
                        level -= 1;
                    }
                    // nop
                    0x04 => {}
                    // end
                    0x09 if level == 0 => break,
                    0x09 => return Err(DtbError::InvalidStruct),
                    invalid => return Err(DtbError::InvalidStructTok(invalid)),
                }
            }
        }

        // verify that reserved memory is well formed and 'aligned'
        {
            if reserved.offset() % 8 != 0 {
                return Err(DtbError::InvalidHeader);
            }
            let mut stream = reserved;
            loop {
                let address = stream.u64().ok_or(DtbError::InvalidReserved)?;
                let size = stream.u64().ok_or(DtbError::InvalidReserved)?;
                if address == 0 && size == 0 {
                    break;
                }
            }
        }

        Ok(Self {
            header,
            reserved,
            structure,
            strings,
        })
    }

    /// # Safety
    ///
    /// `dtb` must point to a valid device tree
    pub unsafe fn from_ptr(dtb: *const u8) -> Result<Self, DtbError> {
        unsafe {
            let magic = u32::from_be_bytes(dtb.cast::<u32>().read().to_ne_bytes());
            if magic != DtbHeader::MAGIC {
                return Err(DtbError::InvalidMagic(magic));
            }
            let total_size = u32::from_be_bytes(dtb.add(4).cast::<u32>().read().to_ne_bytes());
            let Ok(total_size) = total_size.try_into() else {
                return Err(DtbError::InvalidHeader);
            };
            Self::from_slice(core::slice::from_raw_parts(dtb, total_size))
        }
    }

    pub fn reserved(&self) -> DtbReserveIter<'a> {
        DtbReserveIter::new(self.reserved)
    }

    pub fn strings(&self) -> DtbStrings<'a> {
        self.strings
    }

    pub fn structure(&self) -> DtbStructParser<'a> {
        DtbStructParser::new(self.structure, self.strings)
    }

    pub fn root(&self) -> DtbNode<'a> {
        let mut stream = self.structure;
        _ = stream.u32();
        let name = stream.cstr().unwrap_or_default();
        stream.align(4);
        DtbNode::new(name, DtbStructParser::new(self.structure, self.strings))
    }

    pub fn nodes(&self) -> DtbRecursiveNodeIter<'a> {
        DtbRecursiveNodeIter::new(self.structure())
    }

    pub fn header(&self) -> &DtbHeader {
        &self.header
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DtbNode<'a> {
    name: &'a CStr,
    body: DtbStructParser<'a>,
}

impl<'a> DtbNode<'a> {
    pub fn new(name: &'a CStr, body: DtbStructParser<'a>) -> Self {
        Self { name, body }
    }

    pub fn name(&self) -> &'a CStr {
        self.name
    }

    pub fn childern(&self) -> DtbNodeIter<'a> {
        DtbNodeIter::new(self.body)
    }

    pub fn childern_recursive(&self) -> DtbRecursiveNodeIter<'a> {
        DtbRecursiveNodeIter::new(self.body)
    }

    pub fn properties(&self) -> DtbPropertyIter<'a> {
        DtbPropertyIter::new(self.body)
    }
}
