use core::alloc::{GlobalAlloc, Layout};

pub mod buddy;
pub mod page_vec;
pub mod slab;

pub extern crate alloc;
pub use alloc::*;

use crate::{alloc::slab::SlabAllocator, arch::page::Page, mem::Pointer, sync::mutex::CriticalSpinLock};

#[global_allocator]
pub static KALLOC: Kalloc = Kalloc::new();

pub struct Kalloc {}

impl Kalloc {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for Kalloc {
    fn default() -> Self {
        Self::new()
    }
}

static SLAB: CriticalSpinLock<SlabAllocator> = CriticalSpinLock::new(SlabAllocator::new());

unsafe impl GlobalAlloc for Kalloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if layout.size() <= core::mem::size_of::<Page>()/2
            && layout.align() < core::mem::align_of::<Page>()/2
        {
            SLAB.lock().alloc(layout)
        } else {
           unsafe {
                crate::mem::pages::pages_zeroed(
                    layout.size().div_ceil(core::mem::size_of::<Page>()),
                )
                .virt()
                .cast()
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if layout.size() <= core::mem::size_of::<Page>()/2
            && layout.align() < core::mem::align_of::<Page>()/2
        {
            unsafe{
                SLAB.lock().free(ptr, layout)
            }
        } else {
            unsafe {
                crate::mem::pages::free_pages_contiguous(
                    Pointer::from_virt(ptr.cast()),
                    layout.size().div_ceil(core::mem::size_of::<Page>()),
                );
            }
        }
    }
}

/// # Safety
/// .
pub unsafe fn init() {
    SLAB.lock().add_cache(Layout::new::<usize>());
    SLAB.lock().add_cache(Layout::new::<[usize; 2]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 3]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 4]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 6]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 8]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 12]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 16]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 24]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 32]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 48]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 64]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 128]>());
    SLAB.lock().add_cache(Layout::new::<[usize; 256]>());
}
