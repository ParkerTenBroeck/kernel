use crate::dtb::*;

pub enum DumpError{
    Dtb(DtbError),
    Fmt(core::fmt::Error)
}

impl From<core::fmt::Error> for DumpError{
    fn from(val: core::fmt::Error) -> Self {
        DumpError::Fmt(val)
    }
}

impl From<DtbError> for DumpError{
    fn from(value: DtbError) -> Self {
        Self::Dtb(value)
    }
}

pub fn dump<W: core::fmt::Write>(mut out: W, dtb: &Dtb) -> Result<(), DumpError> {
    writeln!(out, "{:#?}", dtb.header()?)?;

    let mut parser = dtb.reserved_parser()?;
    while let Some(reserved) = parser.next()? {
        writeln!(out, "{reserved:?}")?;
    }

    let mut indent = 0;
    let mut parser = dtb.struct_parser()?;
    while let Some(tok) = parser.next()? {
        match tok {
            Tok::BeginNode(name) => {
                for _ in 0..indent {
                    write!(out, "\t")?;
                }
                writeln!(out, "{name:?} {{")?;
                indent += 1;
            }
            Tok::EndNode => {
                indent -= 1;
                for _ in 0..indent {
                    write!(out, "\t")?;
                }
                writeln!(out, "}}")?;
            }
            Tok::Prop(Property { name, mut data }) => {
                for _ in 0..indent {
                    write!(out, "\t")?;
                }
                write!(out, "{name:?} = ")?;
                if writeable_strs(data) {
                    write!(out, "<")?;
                    while !data.is_empty() {
                        write!(out, "{:?}", data.cstr()?)?;
                        if !data.is_empty() {
                            write!(out, " ")?;
                        }
                    }
                    write!(out, ">")?;
                } else if data.len() % 4 == 0 {
                    write!(out, "[")?;
                    while !data.is_empty() {
                        write!(out, "{:#08x}", data.u32()?)?;
                        if !data.is_empty() {
                            write!(out, " ")?;
                        }
                    }
                    write!(out, "]")?;
                } else {
                    write!(out, "[")?;
                    while !data.is_empty() {
                        write!(out, "{:#02x}", data.u8()?)?;
                        if !data.is_empty() {
                            write!(out, " ")?;
                        }
                    }
                    write!(out, "]")?;
                }
                writeln!(out)?;
            }
            Tok::Nop => {}
        }
    }

    fn writeable_strs(mut stream: ByteStream<'_>) -> bool {
        loop {
            if stream.is_empty() {
                return true;
            }
            match stream.cstr() {
                Ok(s_ref) => {
                    if s_ref.is_empty() {
                        return false;
                    }
                }
                Err(_) => return false,
            }
        }
    }

    Ok(())
}
