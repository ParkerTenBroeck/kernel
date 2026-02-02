use core::ffi::CStr;

use crate::dtb::ByteStream;

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

    pub fn get(&self, offset: usize) -> Option<&'a CStr> {
        let mut stream = ByteStream::new(self.buff, 0);
        stream.bytes(offset)?;
        stream.cstr()
    }
}
