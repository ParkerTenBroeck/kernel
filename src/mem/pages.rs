use core::alloc::Layout;

use crate::{alloc::buddy::Buddy, arch::page::PageTable, mem::Pointer, sync::mutex::CriticalSpinLock};


pub static BUDDY: CriticalSpinLock<Buddy> = CriticalSpinLock::new(Buddy::new());

pub type Page = Pointer<PageTable>;

/// # Safety
/// .
pub unsafe fn page_zeroed() -> Page {
    let page = BUDDY
        .lock()
        .alloc(Layout::new::<PageTable>())
        .cast::<PageTable>();

    unsafe {
        page.write_bytes(0, 1);
    }
    Page::from_virt(page)
}

/// # Safety
/// .
pub unsafe fn free_page(page: Page){
    unsafe{
        BUDDY.lock().free_order(page.virt().cast(), 1<<12, 0);
    }
}

/// # Safety
/// .
pub unsafe fn discover(dtb: &crate::dtb::Dtb) {
    use crate::println;
    
    unsafe{
        BUDDY.lock().clear();
    }

    let dtb_range = dtb.slice().as_ptr() as usize..dtb.slice().as_ptr() as usize + dtb.slice().len();
    let mem = crate::mem::physical_region(dtb);
    let kernel_layout_phys = crate::mem::KernelLayout::new();

    let reserved = |page: core::ops::Range<usize>| {
        crate::mem::reserved_regions(dtb)
            .chain([kernel_layout_phys.total.clone(), dtb_range.clone()])
            .any(|reserved| (page.start < reserved.end) & (reserved.start < page.end))
    };

    let page_size = core::mem::size_of::<PageTable>();

    let mut curr_phys_addr = mem.start;
    while curr_phys_addr < mem.end {
        if reserved(curr_phys_addr..curr_phys_addr + page_size) {
            curr_phys_addr += page_size;
            continue;
        }

        let mut page_size = page_size << 1;
        while curr_phys_addr + page_size <= mem.end
            && !reserved(curr_phys_addr..curr_phys_addr + page_size)
        {
            page_size <<= 1;
        }
        page_size >>= 1;

        println!("freeing pages in range {curr_phys_addr:#x?}..{:#x?}", curr_phys_addr + page_size);

        unsafe{
            BUDDY
                .lock()
                .free_region((curr_phys_addr + super::PHYS_ADDR_OFFSET) as *mut u8, page_size);
        }

        curr_phys_addr += page_size;
    }

    println!("Memory discovery complete");
}