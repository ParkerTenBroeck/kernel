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
pub mod hexdump;

use core::ffi::CStr;

use crate::dtb::Dtb;


fn test_pci(){
    let Some((device, _)) = pci::pci().find_device_vendor(0x1b36, 0x05) else {
        println!("test pci device not found");
        return;
    };

    unsafe{
        let (_, command) = pci::pci().read_cmd_status(device);

        pci::pci().write_cmd_status(device, 
            *command.clone()
            .set(pci::CommandRegister::IO_SPACE, false)
            .set(pci::CommandRegister::MEMORY_SPACE, false)
        );

        pci::pci().allocate_bar(device, 0);

        pci::pci().write_cmd_status(device, 
            *command.clone()
            .set(pci::CommandRegister::IO_SPACE, true)
            .set(pci::CommandRegister::MEMORY_SPACE, true)
        );



        let addr = pci::pci().read_bar(device, 0).address() as *mut ();
        

        for i in 0..=255{
            addr.byte_add(0).cast::<u8>().write_volatile(i);
            addr.byte_add(1).cast::<u8>().write_volatile(4);

            let offset = addr.byte_add(4).cast::<u32>().read_volatile();
            let data = addr.byte_add(8).cast::<u32>().read_volatile();

            addr.byte_add(offset as usize).cast::<u32>().write_volatile(data);

            let count = addr.byte_add(12).cast::<u32>().read_volatile();
            let name = addr.byte_add(16).cast::<u8>();
            println!("offset: {offset:x}, data: {data:x}, count: {count},{:?}", CStr::from_ptr(name));
            if offset == 0 {
                break;
            }
        }
    }

}

/// # Safety
/// dtb_ptr must point to a valid dtb tree 
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_main(hart_id: usize, dtb_ptr: *const u8) -> ! {
    uart::early(); // minimal for 16550: optional init
    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    
    _ = dump(&dtb);
    pci::init(&dtb).unwrap();

    test_pci();

    uart::init(&dtb).unwrap(); 

    println!("hart_id = {hart_id}"); 
    syscon::init(&dtb).unwrap();


    // // storage_device::init(&dtb).unwrap();
    vga::init();

    // syscon::poweroff();
    arch::halt()
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
