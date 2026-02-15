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

unsafe impl Send for Buddy {}

struct Block {
    next: Option<NonNull<Block>>,
}

const fn order_size(order: usize) -> usize {
    order_align(order)
}

const fn order_align(order: usize) -> usize {
    1 << (order + MIN_SIZE_P2)
}

const fn bottom_mask(order: usize) -> usize {
    order_align(order) - 1
}

const fn top_mask(order: usize) -> usize {
    !bottom_mask(order)
}

fn layout_order(layout: Layout) -> usize {
    MIN_SIZE_P2.min(layout.align().trailing_zeros() as usize) - MIN_SIZE_P2
}

fn order_of(value: usize) -> usize {
    value.trailing_zeros().max(MIN_SIZE_P2 as u32) as usize - MIN_SIZE_P2
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

    /// # Safety
    ///
    /// .
    pub unsafe fn clear(&mut self) {
        self.free_area = [const { None }; MAX_ORDER];
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        self.alloc_order(layout.size(), layout_order(layout))
    }

    /// # Safety
    ///
    /// .
    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn free(&mut self, data: *mut u8, layout: Layout) {
        self.free_order(data, layout.size(), layout_order(layout))
    }

    /// # Safety
    ///
    /// .
    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn free_region(&mut self, data: *mut u8, size: usize) {
        let offset = data.align_offset(order_align(0));
        let data = data.byte_add(offset);
        let size = size.saturating_sub(offset) & top_mask(0);
        self.free_order(data, size, 0);
    }

    pub fn alloc_order(&mut self, size: usize, order: usize) -> *mut u8 {
        if order > MAX_ORDER {
            panic!("Allocation order too large: {order} max {MAX_ORDER}")
        }

        // TODO finding a better way to allocate sqeuential blocks of smaller orders might be nice

        let requested_order = order_of(size).max(order);

        for (mut block_order, start_place) in
            self.free_area.iter_mut().enumerate().skip(requested_order)
        {
            let Some(mut block) = *start_place else {
                continue;
            };
            unsafe {
                // remove block from list
                *start_place = block.as_mut().next;
            }

            // split block in half until it is desired size
            while block_order != requested_order {
                block_order -= 1;
                let mut rhs = unsafe { block.byte_add(order_size(block_order)) };
                unsafe { rhs.as_mut().next = self.free_area[block_order] }
                self.free_area[block_order] = Some(rhs);
            }
            return block.as_ptr().cast();
        }
        core::ptr::null_mut()
    }

    /// # Safety
    ///
    /// .
    pub unsafe fn free_order(&mut self, mut data: *mut u8, size: usize, order: usize) {
        if order > MAX_ORDER {
            panic!("Allocation order too large: {order} max {MAX_ORDER}")
        }

        let mut size = size & top_mask(0);

        while size > 0 {
            let order = order_of(data as usize).min(order_of(size));

            unsafe {
                self.free_order_exact(data, order);
                data = data.byte_add(order_size(order));
            }
            
            size -= order_size(order);
        }
    }

    unsafe fn free_order_exact(&mut self, data: *mut u8, mut order: usize) {
        let mut block = NonNull::new(data).expect("Null Block").cast::<Block>();

        if block.as_ptr() as usize & bottom_mask(order) != 0 {
            panic!(
                "Unaligned block expected alignment of order 2^{}: {block:?}",
                order + MIN_SIZE_P2
            )
        }

        'outer: for mut current_ptr_place in &mut self.free_area[order..] {
            loop {
                let Some(current_ptr_value) = *current_ptr_place else {
                    // end of of free list
                    unsafe { (*block.as_ptr()).next = None }
                    *current_ptr_place = Some(block);
                    break 'outer;
                };

                if current_ptr_value == block {
                    panic!("Double free on {block:?}")
                }

                let next_ptr_pace =
                    unsafe { &mut current_ptr_value.as_ptr().as_mut().unwrap_unchecked().next };

                // combine two 'buddies' and try to merge higher up
                if current_ptr_value.as_ptr() as usize
                    == block.as_ptr() as usize ^ order_size(order)
                {
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
