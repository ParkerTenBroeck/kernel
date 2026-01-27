use super::DtbError;

use core::ffi::CStr;

#[derive(Clone, Copy, Debug)]
pub struct ByteStream<'a>(&'a [u8], usize);

impl<'a> ByteStream<'a> {
    pub fn new(buf: &'a [u8], offset: usize) -> Self {
        Self(buf, offset)
    }

    pub fn limit(self, size: usize) -> Result<Self, DtbError> {
        match self.0.get(..size) {
            Some(limited) => Ok(Self(limited, self.1)),
            None => Err(DtbError::ByteStream),
        }
    }

    pub fn chunk_ref<const N: usize>(&mut self) -> Result<&[u8; N], DtbError> {
        let chunk;
        (chunk, self.0) = self.0.split_first_chunk::<N>().ok_or(DtbError::ByteStream)?;
        self.1 += N;
        Ok(chunk)
    }

    pub fn chunk<const N: usize>(&mut self) -> Result<[u8; N], DtbError> {
        self.chunk_ref().copied()
    }

    pub fn u8(&mut self) -> Result<u8, DtbError> {
        self.chunk::<1>().map(|c| c[0])
    }

    pub fn u72_array<const N: usize>(&mut self) -> Result<[u8; N], DtbError> {
        let mut array = [0; N];
        for el in &mut array{
            *el = self.u8()?
        }
        Ok(array)
    }

    pub fn u16(&mut self) -> Result<u16, DtbError> {
        self.chunk().map(u16::from_be_bytes)
    }

    pub fn u16_array<const N: usize>(&mut self) -> Result<[u16; N], DtbError> {
        let mut array = [0; N];
        for el in &mut array{
            *el = self.u16()?;
        }
        Ok(array)
    }

    pub fn u32(&mut self) -> Result<u32, DtbError> {
        self.chunk().map(u32::from_be_bytes)
    }

    pub fn u32_array<const N: usize>(&mut self) -> Result<[u32; N], DtbError> {
        let mut array = [0; N];
        for el in &mut array{
            *el = self.u32()?;
        }
        Ok(array)
    }

    pub fn u64(&mut self) -> Result<u64, DtbError> {
        self.chunk().map(u64::from_be_bytes)
    }

    pub fn u64_array<const N: usize>(&mut self) -> Result<[u64; N], DtbError> {
        let mut array = [0; N];
        for el in &mut array{
            *el = self.u64()?;
        }
        Ok(array)
    }

    pub fn align(&mut self, align: usize) -> Result<(), DtbError> {
        let offset = (align - (self.1 & (align - 1))) & (align - 1);
        self.bytes(offset).map(|_| ())
    }

    pub fn cstr(&mut self) -> Result<&'a CStr, DtbError> {
        let str = CStr::from_bytes_until_nul(self.0).map_err(|_| DtbError::ByteStream)?;
        if let Some(rem) = &self.0.get(str.count_bytes() + 1..) {
            self.0 = rem;
        } else {
            return Err(DtbError::ByteStream);
        }
        self.1 += str.count_bytes() + 1;
        Ok(str)
    }

    pub fn bytes(&mut self, length: usize) -> Result<&'a [u8], DtbError> {
        let Some((arr, rem)) = self.0.split_at_checked(length) else {
            return Err(DtbError::ByteStream);
        };
        self.0 = rem;
        self.1 += length;
        Ok(arr)
    }

    pub fn offset(&self) -> usize {
        self.1
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn buf(&self) -> &'a [u8] {
        self.0
    }
}
