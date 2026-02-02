use crate::{dtb::*, pci::pci, println};

#[derive(Debug, Clone, Copy)]
pub enum Width {
    U8,
    U16,
    U32,
}

#[derive(Debug)]
pub struct Flash {
    base: *mut u8,
    size: usize,
    width: Width,
}

impl Flash {
    pub unsafe fn write_u8_word(&mut self, addr: usize, val: u8) {
        unsafe {
            match self.width {
                Width::U8 => self.base.cast::<u8>().add(addr).write_volatile(val),
                Width::U16 => self
                    .base
                    .cast::<u16>()
                    .add(addr)
                    .write_volatile(val as u16 * 0x0101),
                Width::U32 => self
                    .base
                    .cast::<u32>()
                    .add(addr)
                    .write_volatile(val as u32 * 0x0101_0101),
            }
        }
    }

    pub unsafe fn write_u8(&mut self, addr: usize, val: u8) {
        unsafe {
            match self.width {
                Width::U8 => self.base.add(addr).write_volatile(val),
                Width::U16 => self.base.add(addr).write_volatile(val),
                Width::U32 => self.base.add(addr).write_volatile(val),
            }
        }
    }

    pub unsafe fn read_word(&mut self, addr: usize) -> u32 {
        unsafe {
            match self.width {
                Width::U8 => self.base.cast::<u8>().add(addr).read_volatile() as u32,
                Width::U16 => self.base.cast::<u16>().add(addr).read_volatile() as u32,
                Width::U32 => self.base.cast::<u32>().add(addr).read_volatile(),
            }
        }
    }
}

pub fn init(dtb: &Dtb<'_>) {
    println!("Initializing flash");

    let node = dtb
        .nodes()
        .compatible(b"cfi-flash")
        .next()
        .expect("no compatible devices for cfi-flash");

    let props = node.properties();
    let width = props.expect_value(b"bank-width", ByteStream::u32);
    let [base0, size0, base1, size1] = props.expect_value(b"reg", ByteStream::u64_array::<4>);

    let width = match width {
        1 => Width::U8,
        2 => Width::U16,
        4 => Width::U32,
        _ => panic!("Invalid width"),
    };

    let mut f0 = Flash {
        base: base0 as *mut u8,
        size: size0 as usize,
        width,
    };

    let mut f1 = Flash {
        base: base1 as *mut u8,
        size: size1 as usize,
        width,
    };

    println!("{f0:#?}");
    println!("{f1:#?}");

    for (i, f) in [&mut f0, &mut f1].into_iter().enumerate() {
        unsafe {
            f.write_u8_word(0x000055, 0x98);

            let q = f.read_word(0x10) as u8;
            let r = f.read_word(0x11) as u8;
            let y = f.read_word(0x12) as u8;
            if q != b'Q' || r != b'R' || y != b'Y' {
                panic!("flash device {i} did not respond with 'qry'")
            }

            println!("flash device {i}");
            println!("\tcode: {:#x}", f.read_word(0x00));
            println!("\tsize: {:#x}", f.read_word(0x01));
            println!("\tdev size: 2^{}", f.read_word(0x27));
        }
    }

    println!("Setting up PCI storage device");

    let Some((device, id)) = pci().find_device_vendor(0x1af4, 0x1000) else {
        panic!("storage device not found")
    };
}
