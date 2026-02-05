use crate::dtb::{DtbNode, DtbProperties, DtbStructParser, DtbToken};

#[derive(Clone, Copy, Debug)]
pub struct DtbNodeIter<'a>(pub DtbStructParser<'a>, usize);

impl<'a> DtbNodeIter<'a> {
    pub fn new(dtb_struct_parser: DtbStructParser<'a>) -> Self {
        Self(dtb_struct_parser, 0)
    }
}

impl<'a> Iterator for DtbNodeIter<'a> {
    type Item = DtbNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next()? {
                DtbToken::BeginNode(name) if self.1 == 0 => {
                    self.1 += 1;
                    return Some(DtbNode::new(name, self.0));
                }
                DtbToken::BeginNode(_) => self.1 += 1,
                DtbToken::EndNode if self.1 == 0 => return None,
                DtbToken::EndNode => self.1 -= 1,
                DtbToken::Prop(_) => {}
                DtbToken::Nop => {}
            }
        }
    }
}

impl<'a> DtbNodes<'a> for DtbNodeIter<'a> {}

#[derive(Clone, Copy, Debug)]
pub struct DtbRecursiveNodeIter<'a>(pub DtbStructParser<'a>, usize);

impl<'a> DtbRecursiveNodeIter<'a> {
    pub fn new(dtb_struct_parser: DtbStructParser<'a>) -> Self {
        Self(dtb_struct_parser, 0)
    }
}

impl<'a> Iterator for DtbRecursiveNodeIter<'a> {
    type Item = DtbNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next()? {
                DtbToken::BeginNode(name) => {
                    self.1 += 1;
                    return Some(DtbNode::new(name, self.0));
                }
                DtbToken::EndNode if self.1 == 0 => return None,
                DtbToken::EndNode => self.1 -= 1,
                DtbToken::Prop(_) => {}
                DtbToken::Nop => {}
            }
        }
    }
}

impl<'a> DtbNodes<'a> for DtbRecursiveNodeIter<'a> {}

pub trait DtbNodes<'a>: Iterator<Item = DtbNode<'a>> + Sized {
    fn compatible(self, compatible: &[u8]) -> impl Iterator<Item = DtbNode<'a>> {
        self.filter(|node| {
            node.properties()
                .find(b"compatible")
                .is_some_and(|v| v.contains_str(compatible))
        })
    }

    fn nammed(self, name: &[u8]) -> impl Iterator<Item = DtbNode<'a>> {
        self.filter(move |node| node.name.to_bytes() == name)
    }
}
