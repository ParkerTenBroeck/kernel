use core::{alloc::Layout, ptr::NonNull};

use crate::{alloc::page_vec::PageVec, arch::page::Page, mem::Pointer};

#[derive(Debug, Clone)]
struct PageMeta {
    ptr: Pointer<Page>,
    free: usize,
    allocated: usize,
    layout: Layout,
}

#[derive(Debug, Clone)]
struct ListNode {
    next: Option<NonNull<ListNode>>,
}

#[derive(Debug, Clone, Copy)]
struct Cache {
    free_list: Option<NonNull<ListNode>>,
    layout: Layout,
}

#[derive(Debug, Clone)]
pub struct SlabAllocator {
    caches: Caches,
    pages: Pages,
}

unsafe impl Send for SlabAllocator {}
unsafe impl Sync for SlabAllocator {}

#[derive(Debug, Clone)]
struct Caches {
    caches: PageVec<Cache>,
}

impl Caches {
    fn search_cache(&self, layout: Layout) -> Result<usize, usize> {
        self.caches.binary_search_by(|cache| {
            cache
                .layout
                .size()
                .cmp(&layout.size())
                .then(cache.layout.align().cmp(&layout.align()))
        })
    }

    #[inline(never)]
    fn add_cache(&mut self, layout: Layout) {
        let Err(partition) = self.search_cache(layout) else {
            // cache already exists
            return;
        };

        self.caches.insert(
            partition,
            Cache {
                free_list: None,
                layout,
            },
        );
    }

    #[track_caller]
    fn find_closest_cache(&mut self, layout: Layout) -> &mut Cache {
        let (Ok(index) | Err(index)) = self.search_cache(layout);

        if index >= self.caches.len() {
            panic!("Could not find suitable cache for layout {layout:?}")
        }

        for cache in &mut self.caches[index..] {
            if cache.layout.size() >= layout.size() && cache.layout.align() >= layout.align() {
                return cache;
            }
        }

        panic!("Could not find suitable cache for layout {layout:?}")
    }

    #[track_caller]
    fn find_cache(&mut self, layout: Layout) -> &mut Cache {
        let Ok(index) = self.search_cache(layout) else {
            panic!("Could not find suitable cache for layout {layout:?}")
        };

        if index >= self.caches.len() {
            panic!("Could not find suitable cache for layout {layout:?}")
        }

        for cache in &mut self.caches[index..] {
            if cache.layout.size() >= layout.size() && cache.layout.align() >= layout.align() {
                return cache;
            }
        }

        panic!("Could not find suitable cache for layout {layout:?}")
    }
}

#[derive(Debug, Clone)]
struct Pages {
    pages: PageVec<PageMeta>,
}

#[inline(always)]
fn page_base(ptr: *mut u8) -> usize {
    let page_size = core::mem::size_of::<Page>();
    debug_assert!(page_size.is_power_of_two());
    (ptr as usize) & !(page_size - 1)
}

impl Pages {
    pub fn search_page(&self, ptr: Pointer<Page>) -> Result<usize, usize> {
        self.pages
            .binary_search_by(|page| page.ptr.virt().cmp(&ptr.virt()))
    }

    #[track_caller]
    fn allocate_new_page(&mut self, cache: &mut Cache) {
        let slab = unsafe { crate::mem::pages::page_zeroed() };
        let (Ok(index) | Err(index)) = self.search_page(slab);

        self.pages.insert(
            index,
            PageMeta {
                ptr: slab,
                free: core::mem::size_of::<Page>() / cache.layout.size(),
                allocated: 0,
                layout: cache.layout,
            },
        );

        let mut last: Option<NonNull<ListNode>> = None;
        for i in (0..core::mem::size_of::<Page>()).step_by(cache.layout.size()) {
            let node = unsafe { slab.virt().byte_add(i).cast::<ListNode>() };
            let node = NonNull::new(node);
            
            if let Some(last) = last{
                unsafe{
                    (*last.as_ptr()).next = node;
                }
            }
            last = node;
        }
        if let Some(last) = last{
            unsafe{
                last.write(ListNode { next: None });
            }
        }
        cache.free_list = NonNull::new(slab.virt().cast());
    }

    fn remove(&mut self, index: usize) {
        self.pages.remove(index);
    }
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            caches: Caches {
                caches: PageVec::new(),
            },
            pages: Pages {
                pages: PageVec::new(),
            },
        }
    }

    #[track_caller]
    fn round_layout(layout: Layout) -> Layout {
        let align = layout.align().max(8);
        Layout::from_size_align(layout.size().next_multiple_of(align), align).unwrap()
    }

    pub fn add_cache(&mut self, layout: Layout) {
        let layout = SlabAllocator::round_layout(layout);
        self.caches.add_cache(layout);
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let cache = self.caches.find_closest_cache(Self::round_layout(layout));

        if cache.free_list.is_none() {
            self.pages.allocate_new_page(cache);
        }

        if let Some(next) = cache.free_list {
            cache.free_list = unsafe { next.read().next };


            let page_addr = page_base(next.as_ptr().cast());
            let Ok(index) = self
                .pages
                .search_page(Pointer::from_virt(page_addr as *mut Page))
            else {
                panic!("Canot find page {page_addr:x?} in {:#?}", self.pages)
            };
            let page = &mut self.pages.pages[index];
            page.free -= 1;
            page.allocated += 1;


            next.as_ptr().cast()
        } else {
            unreachable!()
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Safety
    ///
    /// .
    #[track_caller]
    pub unsafe fn free(&mut self, ptr: *mut u8, _layout: Layout) {
        let page_addr = page_base(ptr);
        let Ok(index) = self
            .pages
            .search_page(Pointer::from_virt(page_addr as *mut Page))
        else {
            panic!()
        };

        let page = &mut self.pages.pages[index];
        page.free += 1;
        page.allocated -= 1;

        // TODO reclaim completely free pages?
        let cache = self.caches.find_cache(page.layout);
        unsafe {
            ptr.cast::<ListNode>().write(ListNode {
                next: cache.free_list,
            });
            cache.free_list = NonNull::new(ptr.cast())
        }
    }
}

impl Default for SlabAllocator {
    fn default() -> Self {
        Self::new()
    }
}
