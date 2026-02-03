use crate::{
    dtb::{ByteStream, Dtb, DtbProperties},
    println,
};

#[derive(Debug)]
pub struct Region {
    pub start: *const u8,
    pub size: usize,
}

impl Region {
    pub fn new(start: *const u8, end: *const u8) -> Self {
        Self {
            start,
            size: end as usize - start as usize,
        }
    }

    pub fn address_range(&self) -> Range{
        self.start as usize .. self.start as usize + self.size
    }
}

#[derive(Debug)]
pub struct KernelLayout {
    pub text: Region,
    pub ro_data: Region,
    pub data: Region,
    pub bss: Region,
    pub stack: Region,

    pub total: Region,
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
        KernelLayout {
            text: Region::new(&raw const text_start, &raw const text_end),
            ro_data: Region::new(&raw const rodata_start, &raw const rodata_end),
            data: Region::new(&raw const data_start, &raw const data_end),
            bss: Region::new(&raw const bss_start, &raw const bss_end),
            stack: Region::new(&raw const stack_start, &raw const stack_end),
            total: Region::new(&raw const kernel_start, &raw const kernel_end),
        }
    }
}

impl Default for KernelLayout {
    fn default() -> Self {
        Self::new()
    }
}

type Range = core::ops::Range<usize>;

fn subtract_range(a: Range, b: Range) -> [Option<Range>; 2] {
    // No overlap
    if b.end <= a.start || b.start >= a.end {
        return [Some(a), None];
    }

    // B fully covers A
    if b.start <= a.start && b.end >= a.end {
        return [None, None];
    }

    let mut out = [None, None];
    let mut i = 0;

    // Left remainder
    if b.start > a.start {
        out[i] = Some(Range {
            start: a.start,
            end: b.start.min(a.end),
        });
        i += 1;
    }

    // Right remainder
    if b.end < a.end {
        out[i] = Some(Range {
            start: b.end.max(a.start),
            end: a.end,
        });
    }

    out
}

pub fn init(dtb: &Dtb) {
    let kernel_layout = KernelLayout::new();
    println!("{:#x?}", kernel_layout);

    let node = dtb.nodes().find(|node| {
        node.properties()
            .find(b"device_type")
            .is_some_and(|v| v.contains_str(b"memory"))
    }).expect("cannot find 'memory' device");

    let addr_cells = dtb.root().properties().expect_value(b"#address-cells", ByteStream::u32)*4;
    let size_cells = dtb.root().properties().expect_value(b"#size-cells", ByteStream::u32)*4;
    let reg_cells = [addr_cells, size_cells];

    let [start, size] = node.properties().expect_value(b"reg", |stream|stream.usize_cells_arr(reg_cells));

    unsafe{
        let mem = start..start+size;
        
        let mut buddy = crate::alloc::buddy::BUDDY.lock(); 
        
        for range in subtract_range(mem, kernel_layout.total.address_range()).into_iter().flatten(){
            buddy.free_region(range.start as *mut u8, range.end-range.start);
        }
        
        buddy.print();
    }

}
