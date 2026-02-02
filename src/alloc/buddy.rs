use core::{alloc::Layout, ptr::NonNull};

use crate::{print, println, sync::mutex::CriticalSpinLock};

const MIN_SIZE_P2: usize = 12;
const MAX_SIZE_P2: usize = usize::BITS as usize - 1;
pub const MAX_ORDER: usize = MAX_SIZE_P2 - MIN_SIZE_P2;

pub static BUDDY: CriticalSpinLock<Buddy> = CriticalSpinLock::new(Buddy {
    free_area: [const { None }; MAX_ORDER],
});

pub struct Buddy {
    free_area: [Option<NonNull<Block>>; MAX_ORDER],
}

// TODO actually make this thread safe
unsafe impl Send for Buddy {}

struct Block {
    next: Option<NonNull<Block>>,
}

impl Buddy {
    pub fn print(&self) {
        let mut encountered_non_empty = false;
        for (i, mut current) in self.free_area.iter().copied().enumerate().rev() {
            if !encountered_non_empty && current.is_none() {
                continue;
            }
            encountered_non_empty |= current.is_some();
            println!("order: 2^({i}+{MIN_SIZE_P2})");
            print!("\t->");
            while let Some(ptr) = current {
                print!("{ptr:?}->");
                current = unsafe { (*ptr.as_ptr()).next };
            }
            println!("Null")
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let order = MIN_SIZE_P2.min(layout.align().trailing_zeros() as usize) - MIN_SIZE_P2;
        self.alloc_order(layout.size(), order - MIN_SIZE_P2)
    }

    /// # Safety
    ///
    /// .
    pub unsafe fn free(&mut self, data: *mut u8, layout: Layout) {
        let order = MIN_SIZE_P2.min(layout.align().trailing_zeros() as usize) - MIN_SIZE_P2;
        unsafe { self.free_order(data, layout.size(), order) }
    }

    /// # Safety
    ///
    /// .
    pub unsafe fn free_region(&mut self, data: *mut u8, size: usize) {
        unsafe{
            let offset = data.align_offset(1<<MIN_SIZE_P2);
            let data = data.byte_add(offset);
            let size = size.saturating_sub(offset) & !((1<<MIN_SIZE_P2) - 1);
            self.free_order(data, size, 0);
        }
    }

    pub fn alloc_order(&mut self, size: usize, order: usize) -> *mut u8 {
        if order > MAX_ORDER {
            panic!("Allocation order too large: {order} max {MAX_ORDER}")
        }
        let align = 1 << (order + MIN_SIZE_P2);
        //TODO maybe find a better way of doing this
        let size = size.next_multiple_of(1 << order).max(align);

        for start in &self.free_area[order..] {}
        panic!(
            "Unable to allocate {size} bytes of alignment 2^{}",
            order + MIN_SIZE_P2
        );
    }

    /// # Safety
    ///
    /// .
    pub unsafe fn free_order(&mut self, data: *mut u8, size: usize, order: usize) {
        if order > MAX_ORDER {
            panic!("Allocation order too large: {order} max {MAX_ORDER}")
        }
        
        let align = 1 << (order + MIN_SIZE_P2);
        let size = size & !((1<<(MIN_SIZE_P2)) - 1);

        for i in 0..size / align {
            unsafe {
                self.free_order_exact(data.byte_add(i * align), order);
            }
        }
    }

    unsafe fn free_order_exact(&mut self, data: *mut u8, mut order: usize) {
        let align = 1 << (order + MIN_SIZE_P2);
        let mut block = NonNull::new(data).expect("Null Block").cast::<Block>();

        if block.as_ptr() as usize & (align - 1) != 0 {
            panic!(
                "Unaligned block expected alignment of order 2^{}: {block:?}",
                order + MIN_SIZE_P2
            )
        }

        'outer: for mut current_ptr_place in &mut self.free_area[order..] {
            // ensure "next" for the current block is None
            unsafe {
                block.write(Block { next: None });
            }

            loop {
                let align = 1 << (order + MIN_SIZE_P2);

                let Some(current_ptr_value) = *current_ptr_place else {
                    // end of of free list
                    *current_ptr_place = Some(block);
                    break 'outer;
                };

                if current_ptr_value == block {
                    panic!("Double free on {block:?}")
                }

                let next_ptr_pace =
                    unsafe { &mut current_ptr_value.as_ptr().as_mut().unwrap_unchecked().next };

                // combine two 'buddies' and try to merge higher up
                if current_ptr_value.as_ptr() as usize == block.as_ptr() as usize ^ align {
                    order += 1;
                    if order > MAX_ORDER {
                        panic!("Too large: {order} max {MAX_ORDER}")
                    }
                    *current_ptr_place = *next_ptr_pace;
                    block = current_ptr_value.min(block);

                    break;
                }

                current_ptr_place = next_ptr_pace;
            }
        }
    }
}
