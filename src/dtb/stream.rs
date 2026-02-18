use core::ffi::CStr;

#[derive(Clone, Copy, Debug)]
pub struct ByteStream<'a>(&'a [u8], usize);

macro_rules! integer {
    ($self:ident, $name:ident, $name_arr:ident, $name_cell:ident, $name_cell_arr:ident, $name_bytes:ident, $name_bytes_arr:ident, $ty:ty) => {
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


        pub fn $name_cell(&mut $self, cells: u32) -> Option<$ty>{
            $self.$name_bytes(cells*4)
        }

        pub fn $name_cell_arr<const N: usize>(&mut $self, mut cells: [u32; N]) -> Option<[$ty; N]>{
            for cell in &mut cells{
                *cell *= 4;
            }
            $self.$name_bytes_arr(cells)
        }

        pub fn $name_bytes(&mut $self, bytes: u32) -> Option<$ty>{
            let mut value: $ty = 0;
            let backup = *$self;

            for &byte in $self.bytes(bytes as usize)?{
                match value.checked_shl(8){
                    Some(shifted) => {
                        value = shifted | byte as $ty;
                    }
                    None => {
                        *$self = backup;
                        return None
                    }
                }
            }
            Some(value)
        }

        pub fn $name_bytes_arr<const N: usize>(&mut $self, bytes: [u32; N]) -> Option<[$ty; N]>{
            let backup = *$self;
            let mut array = [0; N];
            let mut i = 0;
            while i < N {
                array[i] = match $self.$name_bytes(bytes[i]){
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

    integer!(
        self,
        u8,
        u8_array,
        u8_cells,
        u8_cells_arr,
        u8_bytes,
        u8_bytes_arr,
        u8
    );
    integer!(
        self,
        u16,
        u16_array,
        u16_cells,
        u16_cells_arr,
        u16_bytes,
        u16_bytes_arr,
        u16
    );
    integer!(
        self,
        u32,
        u32_array,
        u32_cells,
        u32_cells_arr,
        u32_bytes,
        u32_bytes_arr,
        u32
    );
    integer!(
        self,
        u64,
        u64_array,
        u64_cells,
        u64_cells_arr,
        u64_bytes,
        u64_bytes_arr,
        u64
    );
    integer!(
        self,
        u128,
        u128_array,
        u128_cells,
        u128_cells_arr,
        u128_bytes,
        u128_bytes_arr,
        u128
    );
    integer!(
        self,
        usize,
        usize_array,
        usize_cells,
        usize_cells_arr,
        usize_bytes,
        usize_bytes_arr,
        usize
    );

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
