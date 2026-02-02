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

pub fn init(dtb: &Dtb) {
    let kernel_layout = KernelLayout::new();
    println!("{:#x?}", kernel_layout);

    let node = dtb.nodes().find(|node| {
        node.properties()
            .find(b"device_type")
            .is_some_and(|v| v.contains_str(b"memory"))
    }).expect("cannot find 'memory' device");

    let [start, size] = node.properties().expect_value(b"reg", ByteStream::u64_array::<2>);

    unsafe{
        let mut buddy = crate::alloc::buddy::BUDDY.lock(); 
        buddy.free_region(&raw const kernel_end as *mut u8, size as usize - kernel_layout.total.size);
        buddy.print();
    }

}
