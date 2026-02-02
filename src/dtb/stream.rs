use core::ffi::CStr;

#[derive(Clone, Copy, Debug)]
pub struct ByteStream<'a>(&'a [u8], usize);

macro_rules! integer {
    ($self:ident, $name: ident, $name_arr: ident, $ty:ty) => {
        pub const fn $name(&mut self) -> Option<$ty> {
            match self.chunk(){
                Some(bytes) => Some(<$ty>::from_be_bytes(bytes)),
                None => None,
            }
        }

        pub const fn $name_arr<const N: usize>(&mut $self) -> Option<[$ty; N]> {
            let backup = *$self;
            let mut array = [0; N];
            let mut i = 0;
            while i < N {
                array[i] = match $self.$name(){
                    Some(value) => value,
                    None => {
                        *$self = backup;
                        return None;
                    }
                };
                i += 1;
            }
            Some(array)
        }
    };
}

impl<'a> ByteStream<'a> {
    pub const fn new(buf: &'a [u8], offset: usize) -> Self {
        Self(buf, offset)
    }

    pub const fn chunk_ref<const N: usize>(&mut self) -> Option<&[u8; N]> {
        let Some((chunk, rem)) = self.0.split_first_chunk::<N>() else {
            return None;
        };
        self.0 = rem;
        self.1 += N;
        Some(chunk)
    }

    pub const fn chunk<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.chunk_ref().copied()
    }

    integer!(self, u8, u8_array, u8);
    integer!(self, u16, u16_array, u16);
    integer!(self, u32, u32_array, u32);
    integer!(self, u64, u64_array, u64);
    integer!(self, u128, u128_array, u128);

    pub const fn align(&mut self, align: usize) {
        let offset = (align - (self.1 & (align - 1))) & (align - 1);
        if self.bytes(offset).is_none() {
            self.1 += offset;
            self.0 = &[];
        }
    }

    pub const fn cstr(&mut self) -> Option<&'a CStr> {
        let Ok(str) = CStr::from_bytes_until_nul(self.0) else {
            return None;
        };
        self.0 = self.0.split_at(str.count_bytes() + 1).1;
        self.1 += str.count_bytes() + 1;
        Some(str)
    }

    pub const fn bytes(&mut self, length: usize) -> Option<&'a [u8]> {
        let Some((arr, rem)) = self.0.split_at_checked(length) else {
            return None;
        };
        self.0 = rem;
        self.1 += length;
        Some(arr)
    }

    pub fn contains_str(mut self, str: &[u8]) -> bool {
        while let Some(next) = self.cstr() {
            if next.to_bytes() == str {
                return true;
            }
        }
        false
    }

    pub const fn offset(&self) -> usize {
        self.1
    }

    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub const fn buf(&self) -> &'a [u8] {
        self.0
    }
}
