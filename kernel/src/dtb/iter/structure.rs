use core::ffi::CStr;

use crate::dtb::*;

#[derive(Clone, Copy, Debug)]
pub struct DtbStructParser<'a> {
    stream: ByteStream<'a>,
    strings: DtbStrings<'a>,
}

#[derive(Clone, Copy, Debug)]
pub enum DtbToken<'a> {
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
}

impl<'a> Iterator for DtbStructParser<'a> {
    type Item = DtbToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        #[allow(clippy::wildcard_in_or_patterns)]
        match self.stream.u32()? {
            // begin node
            0x01 => {
                let str = self.stream.cstr()?;
                self.stream.align(4);
                Some(DtbToken::BeginNode(str))
            }
            // prop
            0x03 => {
                let length = self.stream.u32()?;
                let nameoff = self.stream.u32()?;
                let offset = self.stream.offset();
                let buf = self.stream.bytes(length as usize)?;
                let stream = ByteStream::new(buf, offset);
                self.stream.align(4);
                Some(DtbToken::Prop(Property::new(
                    self.strings.get(nameoff as usize)?,
                    stream,
                )))
            }
            // end node
            0x02 => Some(DtbToken::EndNode),
            // nop
            0x04 => Some(DtbToken::Nop),
            // end
            0x09 | _ => None,
        }
    }
}
