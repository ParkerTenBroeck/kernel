use core::alloc::GlobalAlloc;

pub mod buddy;
pub mod slab;


pub static KALLOC: Kalloc = Kalloc::new();

pub struct Kalloc{
}

impl Kalloc{
    pub const fn new() -> Self{
        Self {  }
    }
}

impl Default for Kalloc {
    fn default() -> Self {
        Self::new()
    }
}


unsafe impl GlobalAlloc for Kalloc{
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        todo!()
    }
}