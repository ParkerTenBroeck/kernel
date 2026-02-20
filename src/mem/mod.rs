pub mod pages;

use crate::dtb::{ByteStream, Dtb, DtbNodes, DtbProperties};

pub const PHYS_ADDR_OFFSET: usize = 0xFFFFFFC000000000;

pub struct Pointer<T>(*mut T);

impl<T> Ord for Pointer<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> PartialOrd for Pointer<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> core::hash::Hash for Pointer<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> Eq for Pointer<T> {}

impl<T> PartialEq for Pointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> core::fmt::Debug for Pointer<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("Pointer").field(&self.0).finish()
    }
}

impl<T> Copy for Pointer<T> {}

impl<T> Clone for Pointer<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Pointer<T> {
    pub fn phys(&self) -> *mut T {
        (self.0 as usize - PHYS_ADDR_OFFSET) as *mut T
    }

    pub fn virt(&self) -> *mut T {
        self.0
    }

    pub fn from_phys(phys: *mut T) -> Self {
        Self((phys as usize + PHYS_ADDR_OFFSET) as *mut T)
    }

    pub fn from_virt(virt: *mut T) -> Self {
        Self(virt)
    }

    pub fn cast<N>(self) -> Pointer<N> {
        Pointer(self.0.cast())
    }
}

#[derive(Debug)]
pub struct KernelLayout {
    pub text: Range,
    pub ro_data: Range,
    pub data: Range,

    pub bss: Range,
    pub stack: Range,

    pub total: Range,
}

unsafe extern "C" {
    #[link_name = "_kernel_start"]
    static kernel_start: u8;
    #[link_name = "_kernel_end"]
    static kernel_end: u8;

    #[link_name = "_text_start"]
    static text_start: u8;
    #[link_name = "_text_end"]
    static text_end: u8;

    #[link_name = "_rodata_start"]
    static rodata_start: u8;
    #[link_name = "_rodata_end"]
    static rodata_end: u8;

    #[link_name = "_data_start"]
    static data_start: u8;
    #[link_name = "_data_end"]
    static data_end: u8;

    #[link_name = "_bss_start"]
    static bss_start: u8;
    #[link_name = "_bss_end"]
    static bss_end: u8;

    #[link_name = "_stack_start"]
    static stack_start: u8;
    #[link_name = "_stack_top"]
    static stack_end: u8;
}

impl KernelLayout {
    pub fn new() -> Self {
        let range = |start, end| start as usize..end as usize;
        KernelLayout {
            text: range(&raw const text_start, &raw const text_end),
            ro_data: range(&raw const rodata_start, &raw const rodata_end),
            data: range(&raw const data_start, &raw const data_end),
            bss: range(&raw const bss_start, &raw const bss_end),
            stack: range(&raw const stack_start, &raw const stack_end),
            total: range(&raw const kernel_start, &raw const kernel_end),
        }
    }
}

impl Default for KernelLayout {
    fn default() -> Self {
        Self::new()
    }
}

type Range = core::ops::Range<usize>;

pub fn reserved_regions(dtb: &Dtb) -> impl Iterator<Item = Range> {
    dtb.root()
        .childern()
        .nammed(b"reserved-memory")
        .filter_map(|node| {
            let addr_cells = dtb
                .root()
                .properties()
                .find_value(b"#address-cells", ByteStream::u32)?;
            let size_cells = dtb
                .root()
                .properties()
                .find_value(b"#size-cells", ByteStream::u32)?;
            Some(node.childern().filter_map(move |reserved| {
                let [start, size] = reserved.properties().find_value(b"reg", |stream| {
                    stream.usize_cells_arr([addr_cells, size_cells])
                })?;

                Some(Range {
                    start,
                    end: start + size,
                })
            }))
        })
        .flatten()
        .chain(dtb.reserved().map(|r| Range {
            start: r.address as usize,
            end: r.address as usize + r.size as usize,
        }))
}

pub fn physical_region(dtb: &Dtb) -> Range {
    let node = dtb
        .nodes()
        .find(|node| {
            node.properties()
                .find(b"device_type")
                .is_some_and(|v| v.contains_str(b"memory"))
        })
        .expect("cannot find 'memory' device");

    let addr_cells = dtb
        .root()
        .properties()
        .expect_value(b"#address-cells", ByteStream::u32);
    let size_cells = dtb
        .root()
        .properties()
        .expect_value(b"#size-cells", ByteStream::u32);
    let reg_cells = [addr_cells, size_cells];

    let [start, size] = node
        .properties()
        .expect_value(b"reg", |stream| stream.usize_cells_arr(reg_cells));

    start..start + size
}