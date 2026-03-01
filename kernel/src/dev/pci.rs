#![allow(clippy::missing_safety_doc)]

use core::{alloc::Layout, cell::UnsafeCell, mem::MaybeUninit};

use crate::{dtb::*, mem::Pointer, println};

#[derive(Clone, Copy, Debug)]
pub struct PciBdf {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
}

#[derive(Clone, Copy, Debug)]
#[allow(unused)]
pub struct PCI {
    dev: *mut u8,
    dev_size: usize,

    bus_range_inc: [u8; 2],

    size_cells: u32,
    interrupt_cells: u32,
    address_cells: u32,

    mm_io_reg: [u32; 2],
    mm_32_reg: [u32; 2],
    mm_64_reg: [u64; 2],

    mm_io_bump: u32,
    mm_32_bump: u32,
    mm_64_bump: u64,
}

mycelium_bitfield::bitfield! {
    #[derive(Eq, PartialEq)]
    pub struct CommandRegister<u16> {
        pub const IO_SPACE: bool;
        pub const MEMORY_SPACE: bool;
        pub const BUS_MASTER: bool;
        pub const SPECIAL_CYCLES: bool;
        pub const MRW_INV: bool;
        pub const VGA_PALETTE_SNOOP: bool;
        pub const PAIRITY_ERR_RESP: bool;
        pub const OARUTY_ERR_RESP: bool;
        pub const _RESERVED0: bool;
        pub const SERR_ENABLE: bool;
        pub const FAST_BTB_ENABLE: bool;
        pub const INTERRUPT_DISABLE: bool;
        const _RESERVED1 = 5;
    }
}

mycelium_bitfield::bitfield! {
    #[derive(Eq, PartialEq)]
    pub struct StatusRegister<u16> {
        pub const _RESERVED0 = 2;
        pub const INTR_STATUS: bool;
        pub const CAPABILITIES_LIST: bool;
        pub const COMPAT_66_MHZ: bool;
        pub const _RESERVED1: bool;
        pub const FAST_BTB_COMPAT: bool;
        pub const MASTER_DATA_PAIRTY_ERROR: bool;
        pub const DEVSEL_TIMING = 2;
        pub const SIGNALED_TARGET_ABORT: bool;
        pub const RECEIVED_TARGET_ABORT: bool;
        pub const RECEIVED_MASTER_ABORT: bool;
        pub const SIGNALED_SYSTEM_ERROR: bool;
        pub const DETECTED_PAIRTY_ERROR: bool;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bar {
    MMIO32(u32, bool),
    MMIO64(u64, bool),
    IO(u32),
}
impl Bar {
    pub fn pointer<T>(self, pci: &PCI) -> Pointer<T> {
        let addr: usize = match self {
            Bar::MMIO32(addr, _) => addr.try_into().unwrap(),
            Bar::MMIO64(addr, _) => addr.try_into().unwrap(),
            Bar::IO(addr) => {
                let addr: usize = addr.try_into().unwrap();
                addr + pci.mm_io_reg[0] as usize
            }
        };

        Pointer::from_phys(addr as *mut T)
    }
}

impl PCI {
    fn ecam_addr(&self, bdf: PciBdf, offset: usize) -> Pointer<u32> {
        debug_assert!(offset < 4096);
        let addr = self.dev as usize
            + ((bdf.bus as usize) << 20)
            + ((bdf.dev as usize) << 15)
            + ((bdf.func as usize) << 12)
            + (offset & !0x3);
        Pointer::from_phys(addr as *mut u32)
    }

    #[inline(always)]
    fn vendor_id(id: u32) -> u16 {
        (id & 0xFFFF) as u16
    }

    #[inline(always)]
    fn device_id(id: u32) -> u16 {
        ((id >> 16) & 0xFFFF) as u16
    }

    pub fn enumerate_devices(&self) {
        for bus in self.bus_range_inc[0]..=self.bus_range_inc[1] {
            for dev in 0..32 {
                for func in 0..8 {
                    let bdf = PciBdf { bus, dev, func };
                    let id = self.ecam_addr(bdf, 0x00);
                    let id = unsafe { id.virt().read_volatile() };

                    let vid = Self::vendor_id(id);
                    let did = Self::device_id(id);

                    if vid == 0xFFFF {
                        if func == 0 {
                            break;
                        }
                        continue;
                    }

                    let cc = unsafe { self.ecam_addr(bdf, 0x08).virt().read_volatile() };
                    let hdr = unsafe { self.ecam_addr(bdf, 0x0C).virt().read_volatile() };
                    let cap = unsafe { self.ecam_addr(bdf, 0x34).virt().read_volatile() };

                    let class = (cc >> 24) as u8;
                    let subclass = (cc >> 16) as u8;
                    let prog_if = (cc >> 8) as u8;

                    let header_type = ((hdr >> 16) & 0xFF) as u8; // bit7 = multifunction
                    let cap_ptr = (cap & 0xFF) as u8;

                    println!("BDF {:02x?} vid={:#06x} did={:#06x}", bdf, vid, did);
                    println!(
                        "\tclass={:#04x} subclass={:#04x} prog_if={:#04x}",
                        class, subclass, prog_if
                    );
                    println!(
                        "\theader_type={:#04x} cap_ptr={:#04x}",
                        header_type, cap_ptr
                    );
                    for i in 0..=5 {
                        println!("\tbar{i}={:x?}", unsafe { self.read_bar(bdf, i) });
                    }
                }
            }
        }
    }

    pub fn find_device_vendor(&self, vendor: u16, device: u16) -> Option<(PciBdf, u16)> {
        for bus in self.bus_range_inc[0]..=self.bus_range_inc[1] {
            for dev in 0..32 {
                for func in 0..8 {
                    let bdf = PciBdf { bus, dev, func };
                    let id = self.ecam_addr(bdf, 0x00);
                    let id = unsafe { id.virt().read_volatile() };

                    let vid = Self::vendor_id(id);
                    let did = Self::device_id(id);

                    if vid == 0xFFFF {
                        if func == 0 {
                            break;
                        }
                        continue;
                    }

                    if vid == vendor && device == did {
                        return Some((bdf, did));
                    }
                }
            }
        }
        None
    }

    pub unsafe fn read_cmd_status(&self, device: PciBdf) -> (StatusRegister, CommandRegister) {
        println!("{:?}", self.pointer(device, 0x04).virt());
        let v = unsafe { self.pointer(device, 0x04).virt().read_volatile() };
        let cmd = (v & 0xFFFF) as u16;
        let status = ((v >> 16) & 0xFFFF) as u16;
        (
            StatusRegister::from_bits(status),
            CommandRegister::from_bits(cmd),
        )
    }

    pub unsafe fn write_cmd_status(&self, device: PciBdf, cmd: CommandRegister) {
        let reg = self.pointer(device, 0x04);
        println!("0: {reg:?}");
        let v = unsafe { reg.virt().read_volatile() };
        println!("1: {reg:?}");
        let v = (v & 0xFFFF_0000) | cmd.bits() as u32;
        unsafe { reg.virt().write_volatile(v) }
        println!("2: {reg:?}");
    }

    pub unsafe fn read_bar(&self, device: PciBdf, bar: u8) -> Bar {
        let off = 0x10 + (bar as usize) * 4;
        let lo = unsafe { self.pointer(device, off).virt().read_volatile() };
        let is_io = (lo & 0x1) != 0;
        if is_io {
            return Bar::IO(lo & !0x3);
        }

        let bar_type = (lo >> 1) & 0x3;
        let is_64 = bar_type == 0x2;
        let prefetchable = lo & 0b1000 != 0;

        let addr_lo = (lo & 0xFFFF_FFF0) as u64;

        if is_64 {
            let hi = unsafe { self.pointer(device, off + 4).virt().read_volatile() } as u64;
            Bar::MMIO64(addr_lo | (hi << 32), prefetchable)
        } else {
            Bar::MMIO32(addr_lo as u32, prefetchable)
        }
    }

    pub unsafe fn bar_alignment(&self, device: PciBdf, bar: u8) -> (u64, Bar) {
        unsafe {
            let value = self.read_bar(device, bar);

            let off = 0x10 + (bar as usize) * 4;
            let align = match value {
                Bar::MMIO64(_, _) => {
                    self.pointer(device, off).virt().write_volatile(0xFFFFFFFF);
                    self.pointer(device, off + 4)
                        .virt()
                        .write_volatile(0xFFFFFFFF);

                    self.pointer(device, off).virt().read_volatile() as u64 & !0b1111
                        | ((self.pointer(device, off + 4).virt().read_volatile() as u64) << 32)
                }
                Bar::MMIO32(_, _) => {
                    self.pointer(device, off).virt().write_volatile(0xFFFFFFFF);
                    self.pointer(device, off).virt().read_volatile() as i32 as i64 as u64 & !0b1111
                }
                Bar::IO(_) => {
                    self.pointer(device, off).virt().write_volatile(0xFFFFFFFF);
                    self.pointer(device, off).virt().read_volatile() as i32 as i64 as u64 & !0b11
                }
            };

            self.write_bar(device, bar, value);

            (align, value)
        }
    }

    pub unsafe fn allocate_bar(&mut self, device: PciBdf, bar: u8) -> Layout {
        match unsafe { self.bar_alignment(device, bar) } {
            (align, Bar::MMIO32(_, b)) => {
                let align = 1u32 << (align.trailing_zeros());
                let addr = self.mm_32_bump + self.mm_32_reg[0];
                let alignment_fix = addr.next_multiple_of(align) - addr;
                let addr = addr + alignment_fix;

                let offset = align + alignment_fix;
                self.mm_32_bump += offset;
                if self.mm_32_bump > self.mm_32_reg[1] {
                    panic!()
                }
                println!(
                    "Allocated {align:#08x} sized region at {addr:#08x} for 32 bit mmio bar {bar} dev {device:?}"
                );
                unsafe {
                    self.write_bar(device, bar, Bar::MMIO32(addr, b));
                }

                Layout::from_size_align(align as usize, align as usize).unwrap()
            }
            (align, Bar::MMIO64(_, b)) => {
                let align = 1u64 << (align.trailing_zeros());
                let addr = self.mm_64_bump + self.mm_64_reg[0];
                let alignment_fix = (align - (addr & (align - 1))) & !(align - 1);
                let addr = addr.next_multiple_of(align) - addr;

                let offset = align + alignment_fix;
                self.mm_64_bump += offset;
                if self.mm_64_bump > self.mm_64_reg[1] {
                    panic!()
                }
                println!(
                    "Allocated {align:#08x} sized region at {addr:#08x} for 64 bit mmio bar {bar} dev {device:?}"
                );
                unsafe {
                    self.write_bar(device, bar, Bar::MMIO64(addr, b));
                }
                Layout::from_size_align(align as usize, align as usize).unwrap()
            }
            (align, Bar::IO(_)) => {
                let align = 1u32 << (align.trailing_zeros());
                let addr = self.mm_io_bump + self.mm_io_reg[0];
                let alignment_fix = addr.next_multiple_of(align) - addr;
                let addr = addr + alignment_fix;

                let offset = align + alignment_fix;
                self.mm_io_bump += offset;
                if self.mm_io_bump > self.mm_io_reg[1] {
                    panic!("IO {align:#x}")
                }
                println!(
                    "Allocated {align:#08x} sized region at {addr:#08x} for io bar {bar} dev {device:?}"
                );
                unsafe {
                    self.write_bar(device, bar, Bar::IO(addr - self.mm_io_reg[0]));
                }

                Layout::from_size_align(align as usize, align as usize).unwrap()
            }
        }
    }

    pub unsafe fn write_bar(&self, device: PciBdf, bar: u8, value: Bar) {
        let off = 0x10 + (bar as usize) * 4;
        unsafe {
            match value {
                Bar::MMIO32(offset, prefetchable) => self
                    .pointer(device, off)
                    .virt()
                    .write_volatile(offset & !0b1111 | ((prefetchable as u32) << 3) | 0b100),
                Bar::IO(offset) => {
                    self.pointer(device, off)
                        .virt()
                        .write_volatile(offset & !0b11 | 0b1);
                }
                Bar::MMIO64(offset, prefetchable) => {
                    self.pointer(device, off).virt().write_volatile(
                        offset as u32 & !0b1111 | ((prefetchable as u32) << 3) | 0b100,
                    );
                    self.pointer(device, off + 4)
                        .virt()
                        .write_volatile((offset >> 32) as u32)
                }
            }
        }
    }

    pub fn pointer(&self, device: PciBdf, offset: usize) -> Pointer<u32> {
        self.ecam_addr(device, offset)
    }
}

struct PCIWrapper(UnsafeCell<MaybeUninit<PCI>>);
unsafe impl Sync for PCIWrapper {}
static PCI: PCIWrapper = PCIWrapper(UnsafeCell::new(MaybeUninit::zeroed()));

pub fn pci() -> &'static mut PCI {
    unsafe { PCI.0.get().as_mut().unwrap_unchecked().assume_init_mut() }
}

pub fn init(dtb: &Dtb<'_>) {
    println!("Initializing PCI");

    let node = dtb
        .nodes()
        .compatible(b"pci-host-ecam-generic")
        .next()
        .expect("no compatible devices for pci-host-ecam-generic");

    let props = node.properties();
    let [start, size] = props.expect_value(b"reg", ByteStream::u64_array::<2>);

    let start_cells = dtb
        .root()
        .properties()
        .expect_value(b"#address-cells", ByteStream::u32);
    let size_cells = props.expect_value(b"#size-cells", ByteStream::u32);
    let interrupt_cells = props.expect_value(b"#interrupt-cells", ByteStream::u32);
    let address_cells = props.expect_value(b"#address-cells", ByteStream::u32);

    let [bus_start, bus_end] = props.expect_value(b"bus-range", ByteStream::u32_array::<2>);

    let meow @ (
        (_addr_io, start_io, size_io),
        (_addr_32, start_32, size_32),
        (_addr_64, start_64, size_64),
    ) = props.expect_value(b"ranges", |stream| {
        Some((
            (
                stream.u128_cells(address_cells)?,
                stream.u32_cells(start_cells)?,
                stream.u32_cells(size_cells)?,
            ),
            (
                stream.u128_cells(address_cells)?,
                stream.u32_cells(start_cells)?,
                stream.u32_cells(size_cells)?,
            ),
            (
                stream.u128_cells(address_cells)?,
                stream.u64_cells(start_cells)?,
                stream.u64_cells(size_cells)?,
            ),
        ))
    });

    println!("{meow:#x?}");

    unsafe {
        let pci = PCI {
            dev: start as *mut u8,
            dev_size: size as usize,
            bus_range_inc: [bus_start as u8, bus_end as u8],
            size_cells,
            interrupt_cells,
            address_cells,

            mm_io_reg: [start_io, size_io],
            mm_32_reg: [start_32, size_32],
            mm_64_reg: [start_64, size_64],

            mm_io_bump: 0,
            mm_32_bump: 0,
            mm_64_bump: 0,
        };

        let device = crate::pci::PciBdf {
            bus: 0,
            dev: 0,
            func: 0,
        };
        let (_, cmd) = pci.read_cmd_status(device);
        pci.write_cmd_status(
            device,
            *cmd.clone()
                .set(CommandRegister::IO_SPACE, true)
                .set(CommandRegister::MEMORY_SPACE, true),
        );

        println!("{pci:#x?}");

        PCI.0.get().write(MaybeUninit::new(pci));
    }

    println!("Initialized PCI");

    pci().enumerate_devices();
}
