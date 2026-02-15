use core::alloc::Layout;

use crate::{
    arch::page::{PageTable, PageTableEntry},
    dtb::{ByteStream, Dtb, DtbNodes, DtbProperties},
};


/// # Safety
/// .
pub unsafe fn page_zeroed() -> *mut PageTable {
    let page = crate::alloc::buddy::BUDDY
        .lock()
        .alloc(Layout::new::<PageTable>())
        .cast::<PageTable>();

    unsafe {
        page.write_bytes(0, 1);
    }
    page
}

// #[allow(unsafe_op_in_unsafe_fn)]
// /// # Safety
// /// .
// pub unsafe fn map_pages(virt: usize, phys: usize, size: usize, entry: PageTableEntry) {
//     for p in 0..((size + 0x0FFF) >> 12) {
//         map_page(virt + (p << 12), phys + (p << 12), entry);
//     }
// }

// #[allow(static_mut_refs)]
// /// # Safety
// /// .
// pub unsafe fn map_page(virt: usize, phys: usize, entry: PageTableEntry) {
//     let ppn2 = (virt >> (9 + 9 + 12)) & ((1 << 9) - 1);
//     let ppn1 = (virt >> (9 + 12)) & ((1 << 9) - 1);
//     let ppn0 = (virt >> (12)) & ((1 << 9) - 1);

//     let mut curr = unsafe { &mut ROOT_PAGE };

//     for ppn in [ppn2, ppn1] {
//         if !curr.entries[ppn].valid() || curr.entries[ppn].is_leaf() {
//             let new = unsafe { page_zeroed() };

//             curr.entries[ppn] = PageTableEntry::new()
//                 .set_valid(true)
//                 .set_ppn(new as u64 >> 12);
//         }
//         curr = unsafe { &mut *((curr.entries[ppn].ppn() << 12) as *mut PageTable) };
//     }

//     curr.entries[ppn0] = entry.set_ppn(phys as u64 >> 12);
// }

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
