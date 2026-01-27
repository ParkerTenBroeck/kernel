use core::{cell::UnsafeCell, mem::MaybeUninit};

use crate::{dtb::*, println};


#[derive(Clone, Copy, Debug)]
pub struct PciBdf {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
}

#[derive(Clone, Copy)]
pub struct PCI{
    dev: *mut u8,
    dev_size: usize,

    bus_range_inc: [u8; 2],

    size_cells: u32,
    interrupt_cells: u32,
    address_cells: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bar{
    MMIO32(u32),
    MMIO64(u64),
    Io
}
impl Bar {
    pub fn expect_mmio(self) -> usize {
        match self{
            Bar::MMIO32(addr) => addr.try_into().unwrap(),
            Bar::MMIO64(addr) => addr.try_into().unwrap(),
            Bar::Io => panic!("expected mmio not io"),
        }
    }
}

impl PCI {
    fn ecam_addr(&self, bdf: PciBdf, offset: usize) -> *mut u32 {
        debug_assert!(offset < 4096);
        let addr = self.dev as usize
            + ((bdf.bus as usize) << 20)
            + ((bdf.dev as usize) << 15)
            + ((bdf.func as usize) << 12)
            + (offset & !0x3);
            addr as *mut u32
    }

    #[inline(always)]
    fn vendor_id(id: u32) -> u16 { (id & 0xFFFF) as u16 }

    #[inline(always)]
    fn device_id(id: u32) -> u16 { ((id >> 16) & 0xFFFF) as u16 }

    pub fn enumerate_devices(&self){
        for bus in self.bus_range_inc[0]..=self.bus_range_inc[1]{
            for dev in 0..32{
                for func in 0..8{
                    let bdf = PciBdf { bus, dev, func };
                    let id = self.ecam_addr(bdf, 0x00);
                    let id = unsafe { id.read_volatile() };

                    let vid = Self::vendor_id(id);
                    let did = Self::device_id(id);

                    if vid == 0xFFFF {
                        if func == 0 { break; }
                        continue;
                    }

                    let cc   = unsafe { self.ecam_addr(bdf, 0x08).read_volatile() };
                    let hdr  = unsafe { self.ecam_addr(bdf, 0x0C).read_volatile() };
                    let bar0 = unsafe { self.ecam_addr(bdf, 0x10).read_volatile() };
                    let cap  = unsafe { self.ecam_addr(bdf, 0x34).read_volatile() };

                    let class = (cc >> 24) as u8;
                    let subclass = (cc >> 16) as u8;
                    let prog_if = (cc >> 8) as u8;

                    let header_type = ((hdr >> 16) & 0xFF) as u8; // bit7 = multifunction
                    let cap_ptr = (cap & 0xFF) as u8;

                    println!("BDF {:02x?} vid={:#06x} did={:#06x}", bdf, vid, did);
                    println!("\tclass={:#04x} subclass={:#04x} prog_if={:#04x}", class, subclass, prog_if);
                    println!("\theader_type={:#04x} cap_ptr={:#04x}", header_type, cap_ptr);
                    println!("\tbar0={:#010x} (io? {})", bar0, (bar0 & 1) != 0);
                }
            }
        }
    }

    pub fn find_device_vendor(&self, vendor: u16) -> Option<(PciBdf, u16)>{
        for bus in self.bus_range_inc[0]..=self.bus_range_inc[1]{
            for dev in 0..32{
                for func in 0..8{
                    let bdf = PciBdf { bus, dev, func };
                    let id = self.ecam_addr(bdf, 0x00);
                    let id = unsafe { id.read_volatile() };

                    let vid = Self::vendor_id(id);
                    let did = Self::device_id(id);

                    if vid == 0xFFFF {
                        if func == 0 { break; }
                        continue;
                    }

                    if vid == vendor {
                        return Some((bdf, did));
                    }
                }
            }
        }
        None
    }

    /// .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn read_bar(&self, device: PciBdf, bar: u8) -> Bar{
        let off = 0x10 + (bar as usize) * 4;
        let lo = unsafe {self.pointer(device, off).read_volatile()};
        let is_io = (lo & 0x1) != 0;
        if is_io {
            // I/O BAR (not usable on RISC-V)
            return Bar::Io;
        }

        let bar_type = (lo >> 1) & 0x3;
        let is_64 = bar_type == 0x2;

        let addr_lo = (lo & 0xFFFF_FFF0) as u64;

        if is_64 {
            let hi = unsafe { self.pointer(device, off+4).read_volatile() } as u64;
            Bar::MMIO64(addr_lo | (hi << 32))
        } else {
            Bar::MMIO32(addr_lo as u32)
        }
    }

    pub fn pointer(&self, device: PciBdf, offset: usize) -> *mut u32{
        self.ecam_addr(device, offset)
    }
}

struct PCIWrapper(UnsafeCell<MaybeUninit<PCI>>);
unsafe impl Sync for PCIWrapper{}
static PCI: PCIWrapper = PCIWrapper(UnsafeCell::new(MaybeUninit::zeroed()));

pub fn pci() -> &'static PCI{
    unsafe{
        PCI.0.get().as_ref().unwrap_unchecked().assume_init_ref()
    }
}

pub fn init(dtb: &Dtb<'_>) -> Result<(), DtbError> {
    println!("Initializing PCI");

    let node = dtb.find_compatable_nodes(b"pci-host-ecam-generic")?.expect_one()?;

    let props = node.properties()?;
    let [start, size] = props.expect(b"reg")?.u64_array::<2>()?;

    let size_cells = props.expect(b"#size-cells")?.u32()?;
    let interrupt_cells = props.expect(b"#interrupt-cells")?.u32()?;
    let address_cells = props.expect(b"#address-cells")?.u32()?;

    let [bus_start, bus_end] = props.expect(b"bus-range")?.u32_array::<2>()?;

    unsafe {
        let pci = PCI { 
            dev: start as *mut u8, 
            dev_size: size as usize, 
            bus_range_inc: [bus_start as u8, bus_end as u8], 
            size_cells, 
            interrupt_cells, 
            address_cells
        };
        PCI.0.get().write(MaybeUninit::new(pci)); 
    }
    
    println!("Initialized PCI");
    
    pci().enumerate_devices();

    Ok(())
}