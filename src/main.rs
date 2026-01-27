#![no_std]
#![no_main]

pub mod dtb;
pub mod uart;
pub mod sout;
pub mod syscon;
pub mod entry;
pub mod panic;
pub mod arch;
pub mod storage_device;
pub mod pci;
pub mod vga;

use crate::dtb::Dtb;




/// # Safety
/// dtb_ptr must point to a valid dtb tree 
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_main(hart_id: usize, dtb_ptr: *const u8) -> ! {
    uart::early(); // minimal for 16550: optional init
    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    
    uart::init(&dtb).unwrap(); 
    _ = dump(&dtb);
    println!("hart_id = {hart_id}"); 
    syscon::init(&dtb).unwrap();

    pci::init(&dtb).unwrap();
    storage_device::init(&dtb).unwrap();

    vga::init();

    syscon::poweroff();
}


pub fn dump(dtb: &Dtb) -> Result<(), dtb::DtbError> {
    use dtb::*;

    println!("{:#?}", dtb.header()?);

    let mut parser = dtb.reserved_parser()?;
    while let Some(reserved) = parser.next()? {
        println!("{reserved:?}");
    }

    let mut indent = 0;
    let mut parser = dtb.struct_parser()?;
    while let Some(tok) = parser.next()? {
        match tok {
            Tok::BeginNode(name) => {
                for _ in 0..indent {
                    print!("\t");
                }
                println!("{name:?} {{");
                indent += 1;
            }
            Tok::EndNode => {
                indent -= 1;
                for _ in 0..indent {
                    print!("\t");
                }
                println!("}}");
            }
            Tok::Prop(Property { name, mut data }) => {
                for _ in 0..indent {
                    print!("\t");
                }
                print!("{name:?} = ");
                if printable_strs(data) {
                    print!("<");
                    while !data.is_empty() {
                        print!("{:?}", data.cstr()?);
                        if !data.is_empty() {
                            print!(" ");
                        }
                    }
                    print!(">");
                } else if data.len() % 4 == 0 {
                    print!("[");
                    while !data.is_empty() {
                        print!("{:#08x}", data.u32()?);
                        if !data.is_empty() {
                            print!(" ");
                        }
                    }
                    print!("]");
                } else {
                    print!("[");
                    while !data.is_empty() {
                        print!("{:#02x}", data.u8()?);
                        if !data.is_empty() {
                            print!(" ");
                        }
                    }
                    print!("]");
                }
                println!()
            }
            Tok::Nop => {}
        }
    }

    fn printable_strs(mut stream: ByteStream<'_>) -> bool {
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
