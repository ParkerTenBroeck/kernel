
use crate::{dtb::*, pci::pci, print, println};

#[derive(Debug, Clone, Copy)]
pub enum Width{
    U8,
    U16,
    U32,
}

#[derive(Debug)]
pub struct Flash{
    base: *mut u8,
    size: usize,
    width: Width,
}

impl Flash{
    pub unsafe fn write_u8_word(&mut self, addr: usize, val: u8){
        unsafe{
            match self.width{
                Width::U8 => self.base.cast::<u8>().add(addr).write_volatile(val),
                Width::U16 => self.base.cast::<u16>().add(addr).write_volatile(val as u16 * 0x0101),
                Width::U32 => self.base.cast::<u32>().add(addr).write_volatile(val as u32 * 0x0101_0101),
            }
        }
    }

    pub unsafe fn write_u8(&mut self, addr: usize, val: u8){
        unsafe{
            match self.width{
                Width::U8 => self.base.add(addr).write_volatile(val),
                Width::U16 => self.base.add(addr).write_volatile(val),
                Width::U32 => self.base.add(addr).write_volatile(val),
            }
        }
    }

    pub unsafe fn read_word(&mut self, addr: usize) -> u32{
        unsafe{
            match self.width{
                Width::U8 => self.base.cast::<u8>().add(addr).read_volatile() as u32,
                Width::U16 => self.base.cast::<u16>().add(addr).read_volatile() as u32,
                Width::U32 => self.base.cast::<u32>().add(addr).read_volatile(),
            }
        }
    }
}

pub fn init(dtb: &Dtb<'_>) -> Result<(), DtbError> {
    println!("Initializing flash");

    let node = dtb.find_compatable_nodes(b"cfi-flash")?.expect_one()?;

    let props = node.properties()?;
    let width = props.expect(b"bank-width")?.u32()?;
    let [base0, size0, base1, size1] = props.expect(b"reg")?.u64_array::<4>()?;

    let width = match width{
        1 => Width::U8,
        2 => Width::U16,
        4 => Width::U32,
        _ => panic!("Invalid width")
    };

    let mut f0 = Flash{
        base:  base0 as *mut u8,
        size: size0 as usize,
        width
    };

    let mut f1 = Flash{
        base:  base1 as *mut u8,
        size: size1 as usize,
        width
    };

    println!("{f0:#?}");
    println!("{f1:#?}");
    
    for (i, f) in [&mut f0, &mut f1].into_iter().enumerate(){
        unsafe{
            f.write_u8_word(0x000055, 0x98);

            let q = f.read_word(0x10) as u8;
            let r = f.read_word(0x11) as u8;
            let y = f.read_word(0x12) as u8;
            if q != b'Q' || r != b'R' || y != b'Y'{
                panic!("flash device {i} did not respond with 'qry'")
            }

            println!("flash device {i}");
            println!("\tcode: {:#x}", f.read_word(0x00));
            println!("\tsize: {:#x}", f.read_word(0x01));
            println!("\tdev size: 2^{}", f.read_word(0x27));
        }
    }

    println!("Setting up PCI storage device");

    let Some((device, id)) = pci().find_device_vendor(0x1af4) else {
        panic!("storage device not found")
    };

    // // enable mmio and busmaster
    // unsafe{
    //     let command = pci().pointer(device, 0x04);

    //     let v = command.read_unaligned();
    //     let cmd = (v & 0xFFFF) as u16;
    //     let new_cmd = cmd | (1 << 1) | (1 << 2);
    //     let new_v = (v & 0xFFFF_0000) | (new_cmd as u32);
    //     command.write_volatile(new_v);
    // }

    // //bar 0
    // let bar0 = 'bar0: {unsafe{
    //     let bar0 = pci().pointer(device, 0x10).read_volatile();
    //     println!("{bar0}");
    //     if (bar0 & 0x1) != 0 {
    //         // I/O BAR (rare on RISC-V virt). Not what we want.
    //         break 'bar0 0;
    //     }
    //     let is_64 = ((bar0 >> 1) & 0x3) == 0x2;
    //     let addr_lo = (bar0 & 0xFFFF_FFF0) as u64;

    //     if is_64 {
    //         let bar1 = pci().pointer(device, 0x14).read_volatile() as u64;
    //         let addr = addr_lo | (bar1 << 32);
    //         break 'bar0 addr;
    //     } else {
    //         break 'bar0 addr_lo;
    //     }
    // }};

    // println!("{bar0:#08?}");

    // println!("storage ID: {device:02x?}");


    Ok(())
}