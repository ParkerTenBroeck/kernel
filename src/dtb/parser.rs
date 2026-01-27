use core::ffi::CStr;

use crate::dtb::*;

#[derive(Clone, Copy, Debug)]
pub struct DtbStrings<'a> {
    buff: &'a [u8],
}

impl<'a> DtbStrings<'a> {
    pub fn new(buff: &'a [u8]) -> Self {
        Self { buff }
    }

    pub fn buf(&self) -> &'a [u8] {
        self.buff
    }

    pub fn get(&self, offset: usize) -> Result<&'a CStr, DtbError> {
        let mut stream = ByteStream::new(self.buff, 0);
        stream.bytes(offset)?;
        stream.cstr()
    }
}

pub struct DtbReserveParser<'a> {
    stream: ByteStream<'a>,
}

impl<'a> DtbReserveParser<'a> {
    pub fn new(stream: ByteStream<'a>) -> Self {
        Self { stream }
    }

    pub fn next(&mut self) -> Result<Option<Reserved>, DtbError> {
        let address = self.stream.u64()?;
        let size = self.stream.u64()?;
        if address == 0 && size == 0 {
            Ok(None)
        } else {
            Ok(Some(Reserved { address, size }))
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DtbStructParser<'a> {
    stream: ByteStream<'a>,
    strings: DtbStrings<'a>,
}

#[derive(Clone, Copy, Debug)]
pub enum Tok<'a> {
    BeginNode(&'a CStr),
    EndNode,
    Prop(Property<'a>),
    Nop,
}

impl<'a> DtbStructParser<'a> {
    pub fn new(stream: ByteStream<'a>, strings: DtbStrings<'a>) -> Self {
        Self { stream, strings }
    }

    pub fn stream(&self) -> ByteStream<'a> {
        self.stream
    }

    pub fn strings(&self) -> DtbStrings<'a> {
        self.strings
    }

    pub fn next(&mut self) -> Result<Option<Tok<'a>>, DtbError> {
        match self.stream.u32()? {
            // begin node
            0x01 => {
                let str = self.stream.cstr()?;
                self.stream.align(4)?;
                Ok(Some(Tok::BeginNode(str)))
            }
            // prop
            0x03 => {
                let length = self.stream.u32()?;
                let nameoff = self.stream.u32()?;
                let offset = self.stream.offset();
                let buf = self.stream.bytes(length as usize)?;
                let stream = ByteStream::new(buf, offset);
                self.stream.align(4)?;
                Ok(Some(Tok::Prop(Property::new(
                    self.strings.get(nameoff as usize)?,
                    stream,
                ))))
            }
            // end node
            0x02 => Ok(Some(Tok::EndNode)),
            // nop
            0x04 => Ok(Some(Tok::Nop)),
            // end
            0x09 => Ok(None),
            val => Err(DtbError::InvalidStructTok(val)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DtbNodePropertyParser<'a>(pub DtbStructParser<'a>);

impl<'a> DtbNodePropertyParser<'a> {
    pub fn new(dtb_struct_parser: DtbStructParser<'a>) -> Self {
        Self(dtb_struct_parser)
    }

    pub fn next(&mut self) -> Result<Option<Property<'a>>, DtbError> {
        loop {
            match self.0.next()? {
                Some(Tok::BeginNode(_)) => return Ok(None),
                Some(Tok::EndNode) => return Ok(None),
                Some(Tok::Prop(property)) => return Ok(Some(property)),
                None => return Ok(None),
                Some(Tok::Nop) => {}
            }
        }
    }

    pub fn find(mut self, name: &[u8]) -> Result<Option<ByteStream<'a>>, DtbError>{
        while let Some(prop) = self.next()?{
            if prop.name.to_bytes() == name{
                return Ok(Some(prop.data))
            }
        }
        Ok(None)
    }

    pub fn expect(mut self, name: &[u8]) -> Result<ByteStream<'a>, DtbError>{
        while let Some(prop) = self.next()?{
            if prop.name.to_bytes() == name{
                return Ok(prop.data)
            }
        }
        Err(DtbError::ExpectedProperty)
    }
}
