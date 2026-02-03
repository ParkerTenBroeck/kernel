use crate::dtb::{ByteStream, DtbStructParser, DtbToken, Property};

#[derive(Clone, Copy, Debug)]
pub struct DtbPropertyIter<'a>(pub DtbStructParser<'a>);

impl<'a> DtbPropertyIter<'a> {
    pub fn new(dtb_struct_parser: DtbStructParser<'a>) -> Self {
        Self(dtb_struct_parser)
    }
}

impl<'a> DtbProperties<'a> for DtbPropertyIter<'a> {}

impl<'a> Iterator for DtbPropertyIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next()? {
                DtbToken::BeginNode(_) => return None,
                DtbToken::EndNode => return None,
                DtbToken::Prop(property) => return Some(property),
                DtbToken::Nop => {}
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DtbRecursivePropertyIter<'a>(pub DtbStructParser<'a>, usize);

impl<'a> DtbRecursivePropertyIter<'a> {
    pub fn new(dtb_struct_parser: DtbStructParser<'a>) -> Self {
        Self(dtb_struct_parser, 0)
    }

    pub fn find_name(self, name: &[u8]) -> Option<ByteStream<'a>> {
        for prop in self {
            if prop.name.to_bytes() == name {
                return Some(prop.data);
            }
        }
        None
    }
}

impl<'a> DtbProperties<'a> for DtbRecursivePropertyIter<'a> {}

impl<'a> Iterator for DtbRecursivePropertyIter<'a> {
    type Item = Property<'a>;

    fn next(&mut self) -> Option<Self::Item> {

        loop {
            match self.0.next()? {
                DtbToken::BeginNode(_) => self.1 += 1,
                DtbToken::EndNode if self.1 == 0 => return None,
                DtbToken::EndNode => self.1 -= 1,
                DtbToken::Prop(property) => return Some(property),
                DtbToken::Nop => {}
            }
        }
    }
}

pub trait DtbProperties<'a>: Iterator<Item = Property<'a>> + Sized {
    fn find(self, name: &[u8]) -> Option<ByteStream<'a>> {
        for prop in self {
            if prop.name.to_bytes() == name {
                return Some(prop.data);
            }
        }
        None
    }

    #[track_caller]
    fn expect(self, name: &[u8]) -> ByteStream<'a> {
        match self.find(name) {
            Some(some) => some,
            None => panic!("Expected property {:?} found None", name),
        }
    }

    fn find_value<T>(
        self,
        name: &[u8],
        parse: impl FnOnce(&mut ByteStream<'a>) -> Option<T>,
    ) -> Option<T> {
        for mut prop in self {
            if prop.name.to_bytes() == name {
                return parse(&mut prop.data);
            }
        }
        None
    }

    #[track_caller]
    fn expect_value<T>(
        self,
        name: &[u8],
        parse: impl FnOnce(&mut ByteStream<'a>) -> Option<T>,
    ) -> T {
        for mut prop in self {
            if prop.name.to_bytes() == name {
                match parse(&mut prop.data) {
                    Some(value) => return value,
                    None => panic!("Malformed property {:?}", name),
                }
            }
        }
        panic!("Expected property {:?} found None", name)
    }
}
