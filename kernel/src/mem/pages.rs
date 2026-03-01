use core::alloc::Layout;

use crate::{
    alloc::buddy::Buddy,
    arch::page::{Page, PageTable},
    mem::Pointer,
    sync::mutex::CriticalSpinLock,
};

pub static BUDDY: CriticalSpinLock<Buddy> = CriticalSpinLock::new(Buddy::new());

pub type PagePtr = Pointer<Page>;

/// # Safety
/// .
pub unsafe fn page_zeroed() -> PagePtr {
    let page = BUDDY
        .lock()
        .alloc(Layout::new::<PageTable>())
        .cast::<Page>();

    if page.is_null() {
        panic!("OOM");
    }

    unsafe {
        page.write_bytes(0, 1);
    }
    Pointer::from_virt(page)
}

/// # Safety
/// .
#[track_caller]
pub unsafe fn pages_zeroed(num: usize) -> PagePtr {
    let page = BUDDY
        .lock()
        .alloc_order(Layout::new::<Page>().size()*num, 0)
        .cast::<Page>();

    if page.is_null() {
        panic!("OOM");
    }

    unsafe {
        page.write_bytes(0, num);
    }
    PagePtr::from_virt(page)
}

/// # Safety
/// .
#[track_caller]
pub unsafe fn free_page(page: PagePtr) {
    unsafe {
        BUDDY.lock().free_region(page.virt().cast(), 1 << 12);
    }
}

/// # Safety
/// .
#[track_caller]
pub unsafe fn free_pages_contiguous(page: PagePtr, num: usize) {
    unsafe {
        BUDDY
            .lock()
            .free_region(page.virt().cast(), num * (1 << 12));
    }
}

/// # Safety
/// .
pub unsafe fn discover(dtb: &crate::dtb::Dtb, vma: usize, pma: usize) {
    use crate::println;

    unsafe {
        BUDDY.lock().clear();
    }

    let dtb_range_phys_start = Pointer::from_virt(dtb.slice().as_ptr().cast_mut()).phys() as usize;
    let dtb_range_phys_end =
        Pointer::from_virt(dtb.slice().as_ptr().cast_mut()).phys() as usize + dtb.slice().len();
    let dtb_range_phys = dtb_range_phys_start..dtb_range_phys_end;

    let mem = crate::mem::physical_region(dtb);
    let kernel_layout_virt = crate::mem::KernelLayout::new();
    println!("virt kernel layout: {kernel_layout_virt:#x?}");
    let kernel_range_phys_start = kernel_layout_virt.total.start - vma + pma;
    let kernel_range_phys_end = kernel_layout_virt.total.end - vma + pma;
    let kernel_range_phys = kernel_range_phys_start..kernel_range_phys_end;

    let reserved = |page: core::ops::Range<usize>| {
        crate::mem::reserved_regions(dtb)
            .chain([kernel_range_phys.clone(), dtb_range_phys.clone()])
            .any(|reserved| (page.start < reserved.end) & (reserved.start < page.end))
    };

    let page_size = core::mem::size_of::<PageTable>();

    println!("kernel region phys: {kernel_range_phys:#x?}");
    println!("dtb    region phys: {dtb_range_phys:#x?}");
    println!("memory region phys: {mem:#x?}");

    // for page in (mem.start..mem.end).step_by(page_size){
    //     if reserved(page..page + page_size) {
    //         continue;
    //     }
    //     println!(
    //         "freeing pages in range {page:#x?}..{:#x?}", page + page_size
    //     );
    //     unsafe{
    //         free_page(PagePtr::from_phys(page as *mut Page));
    //     }
    // }

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

        let start = Pointer::from_phys(curr_phys_addr as *mut u8).virt();

        println!(
            "freeing pages in range {start:#x?}..{:#x?}",
            unsafe{start.byte_add(page_size)}
        );

        unsafe {
            BUDDY.lock().free_region(
                start,
                page_size,
            );
        }

        curr_phys_addr += page_size;
    }

    BUDDY.lock().print();

    println!("Memory discovery complete");
}
