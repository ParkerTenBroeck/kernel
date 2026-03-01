use crate::dtb::*;

pub struct DtbReserveIter<'a> {
    stream: ByteStream<'a>,
}

impl<'a> DtbReserveIter<'a> {
    pub fn new(stream: ByteStream<'a>) -> Self {
        Self { stream }
    }

    pub fn stream(&self) -> ByteStream<'a> {
        self.stream
    }
}

impl<'a> Iterator for DtbReserveIter<'a> {
    type Item = Reserved;

    fn next(&mut self) -> Option<Self::Item> {
        let address = self.stream.u64()?;
        let size = self.stream.u64()?;
        if address == 0 && size == 0 {
            None
        } else {
            Some(Reserved { address, size })
        }
    }
}
