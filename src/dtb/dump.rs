use crate::dtb::*;

pub enum DumpError {
    Dtb(DtbError),
    Fmt(core::fmt::Error),
}

impl From<core::fmt::Error> for DumpError {
    fn from(val: core::fmt::Error) -> Self {
        DumpError::Fmt(val)
    }
}

impl From<DtbError> for DumpError {
    fn from(value: DtbError) -> Self {
        Self::Dtb(value)
    }
}

pub fn dump<W: core::fmt::Write>(mut out: W, dtb: &Dtb) -> Result<(), DumpError> {
    writeln!(out, "{:#?}", dtb.header())?;

    for reserved in dtb.reserved() {
        writeln!(out, "{reserved:?}")?;
    }

    let mut indent = 0;
    for tok in dtb.structure() {
        match tok {
            DtbToken::BeginNode(name) => {
                for _ in 0..indent {
                    write!(out, "\t")?;
                }
                writeln!(out, "{name:?} {{")?;
                indent += 1;
            }
            DtbToken::EndNode => {
                indent -= 1;
                for _ in 0..indent {
                    write!(out, "\t")?;
                }
                writeln!(out, "}}")?;
            }
            DtbToken::Prop(Property { name, mut data }) => {
                for _ in 0..indent {
                    write!(out, "\t")?;
                }
                write!(out, "{name:?} = ")?;
                if writeable_strs(data) {
                    write!(out, "<")?;
                    while let Some(str) = data.cstr() {
                        write!(out, "{:?}", str)?;
                        if !data.is_empty() {
                            write!(out, " ")?;
                        }
                    }
                    write!(out, ">")?;
                } else if data.len() % 4 == 0 {
                    write!(out, "[")?;
                    while let Some(value) = data.u32() {
                        write!(out, "{:#08x}", value)?;
                        if !data.is_empty() {
                            write!(out, " ")?;
                        }
                    }
                    write!(out, "]")?;
                } else {
                    write!(out, "[")?;
                    while let Some(value) = data.u8() {
                        write!(out, "{:#08x}", value)?;
                        if !data.is_empty() {
                            write!(out, " ")?;
                        }
                    }
                    write!(out, "]")?;
                }
                writeln!(out)?;
            }
            DtbToken::Nop => {}
        }
    }

    fn writeable_strs(mut stream: ByteStream<'_>) -> bool {
        loop {
            if stream.is_empty() {
                return true;
            }
            match stream.cstr() {
                Some(s_ref) => {
                    if s_ref.is_empty() {
                        return false;
                    }
                }
                None => return false,
            }
        }
    }

    Ok(())
}
