use core::{fmt::Write, sync::atomic::{AtomicBool, fence}};

use crate::{
    arch::page::{PageTable, PageTableEntry}, dev::uart, dtb::Dtb, kernel_entry, mem::KernelLayout, println, std::stdio
};

static FIRST: AtomicBool = AtomicBool::new(true);

pub static mut TRAMPOLINE_ROOT_PAGE: PageTable = PageTable {
    entries: [PageTableEntry::new(); 512],
};

pub static mut TRAMPOLINE_ROOT_PAGE_L2_LOW: PageTable = PageTable {
    entries: [PageTableEntry::new(); 512],
};

pub static mut TRAMPOLINE_ROOT_PAGE_L2_HIGH: PageTable = PageTable {
    entries: [PageTableEntry::new(); 512],
};

#[inline(never)]
unsafe fn relocate_kernel(addr: usize){
    _ = stdio::sout().write_str("relocating kernel\n");
    unsafe {
        super::reloc::relocate_kernel(addr);
    }

    fence(core::sync::atomic::Ordering::SeqCst);

    crate::stdio::set_sout(early_print);

    println!("relocated kernel to {addr:#x?}");
}

fn early_print(str: &str){
    for byte in str.as_bytes() {
        unsafe { core::arch::asm!("ecall", in("a7") 1, in("a6") 0, in("a0") *byte) }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn setup_vm_trampoline(_: usize, _: *const u8, vma: usize, pma: usize) -> *const PageTable {
    if !FIRST.swap(false, core::sync::atomic::Ordering::SeqCst){
        early_print("\n\nNOT FIRST");
        early_panic();
    }
    crate::stdio::set_sout(early_print);
    relocate_kernel(pma);

    assert!(pma & ((1<<(12+9))-1) == 0, "pma not properly aligned");
    assert!(vma & ((1<<(12+9))-1) == 0, "vma not properly aligned");


    println!("Physical kernel layout: {:#x?}", KernelLayout::new());

    println!("Setting up virtual memory trampoline");

    for i in 0..256 {
        TRAMPOLINE_ROOT_PAGE.entries[i] = (PageTableEntry::COM_DEV.set_executable(true)
            | PageTableEntry::DIRTY_ACCESSED)
            .set_ppn((i as u64) << 18);
    }

    for i in 0..128 {
        TRAMPOLINE_ROOT_PAGE.entries[i+256] = (PageTableEntry::COM_DEV
            | PageTableEntry::DIRTY_ACCESSED)
            .set_ppn((i as u64) << 18);
    }

    for i in 0..512 {
        TRAMPOLINE_ROOT_PAGE_L2_LOW.entries[i] = PageTableEntry::new()
            .set_accessed(true)
            .set_dirty(true)
            .set_readable(true)
            .set_writable(true)
            .set_executable(true)
            .set_valid(true)
            .set_ppn((pma as u64 >> 12) + ((i as u64) << 9));

        TRAMPOLINE_ROOT_PAGE_L2_HIGH.entries[i] = PageTableEntry::new()
            .set_accessed(true)
            .set_dirty(true)
            .set_readable(true)
            .set_writable(true)
            .set_executable(true)
            .set_valid(true)
            .set_ppn((pma as u64 >> 12) + ((i as u64) << 9) + (512<<9))
    }

    TRAMPOLINE_ROOT_PAGE.entries[510] = PageTableEntry::new()
        .set_valid(true)
        .set_ppn(&raw const TRAMPOLINE_ROOT_PAGE_L2_LOW as u64 >> 12);

    TRAMPOLINE_ROOT_PAGE.entries[511] = PageTableEntry::new()
        .set_valid(true)
        .set_ppn(&raw const TRAMPOLINE_ROOT_PAGE_L2_HIGH as u64 >> 12);

    println!("Finished virtual memory trampoline");

    &raw const TRAMPOLINE_ROOT_PAGE
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn setup_vm(_: usize, dtb_ptr: *const u8, vma: usize, pma: usize) {

    crate::stdio::set_sout(early_print);
    relocate_kernel(vma);


    println!("Discovering memory");


    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };

    crate::alloc::buddy::BUDDY.lock().clear();

    let dtb_range = dtb_ptr as usize..dtb_ptr as usize + dtb.header().totalsize as usize;
    let mem = crate::mem::physical_region(&dtb);
    let kernel_layout_phys = crate::mem::KernelLayout::new();

    let reserved = |page: core::ops::Range<usize>| {
        crate::mem::reserved_regions(&dtb)
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

        println!("freeing range {curr_phys_addr:#x?}..{:#x?}", curr_phys_addr + page_size);

        crate::alloc::buddy::BUDDY
            .lock()
            .free_region(curr_phys_addr as *mut u8, page_size);


        curr_phys_addr += page_size;
    }

    println!("Memory discovery complete");

    println!("Creating kernel memory map");

    let virt_offset = vma - pma;

    println!("pma: {pma:#x?}    vma: {vma:#x?}    offset: {virt_offset:#x?}");

    // text section
    // crate::mem::map_pages(
    //     kernel_layout_phys.text.start + virt_offset,
    //     kernel_layout_phys.text.start,
    //     kernel_layout_phys.text.end - kernel_layout_phys.text.start,
    //     PageTableEntry::COM_EXEC | PageTableEntry::DIRTY_ACCESSED,
    // );

    // // ro section
    // crate::mem::map_pages(
    //     kernel_layout_phys.ro_data.start + virt_offset,
    //     kernel_layout_phys.ro_data.start,
    //     kernel_layout_phys.ro_data.end - kernel_layout_phys.ro_data.start,
    //     PageTableEntry::COM_RO | PageTableEntry::DIRTY_ACCESSED,
    // );

    // // data section
    // crate::mem::map_pages(
    //     kernel_layout_phys.data.start + virt_offset,
    //     kernel_layout_phys.data.start,
    //     kernel_layout_phys.data.end - kernel_layout_phys.data.start,
    //     PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
    // );

    // // bss section
    // crate::mem::map_pages(
    //     kernel_layout_phys.bss.start + virt_offset,
    //     kernel_layout_phys.bss.start,
    //     kernel_layout_phys.bss.end - kernel_layout_phys.bss.start,
    //     PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
    // );

    // // stacks
    // crate::mem::map_pages(
    //     kernel_layout_phys.stack.start + virt_offset,
    //     kernel_layout_phys.stack.start,
    //     kernel_layout_phys.stack.end - kernel_layout_phys.stack.start,
    //     PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
    // );

    // _ = PageTable::disp_table(&raw const ROOT_PAGE, 0, 0, uart::uart());

    println!("Completed kernel meory map");
}

unsafe extern "C" fn early_panic(){
    for byte in "\n\nearly panic\n\n".as_bytes() {
        unsafe { core::arch::asm!("ecall", in("a7") 1, in("a6") 0, in("a0") *byte) }
    }
    uart::uart()
        .str("sepc: ").hex(riscv::register::sepc::read())
        .str("\nstval: ").hex(riscv::register::stval::read())
        .str("\nscause: ").hex(riscv::register::scause::read().bits());
    super::halt()
}

core::arch::global_asm!(
    "
        .option push
        .option norelax
.section .text.entry
.global _start
_start:
    lla      gp, __global_pointer$

    // clear bss
    lla a3, _bss_start
    lla a4, _bss_end
    ble a4, a3, .Lclear_bss_done
    .Lclear_bss:
        ld zero, 0(a3)
        add a3, a3, 8
        blt a3, a4, .Lclear_bss
    .Lclear_bss_done:

    
    lga a2, KERNEL_VMA
    lla a3, _kernel_start
    // fixup offset
    sub s0, a2, a3

    lla t0, {panic}
    csrw stvec, t0

    lla sp, _stack_top
    move s1, a0
    move s2, a1
    move s3, a2
    move s4, a3
    call {setup_vm_trampoline}    
    
    // enable virtual memory
    move t1, a0
    srli t1, t1, 12
    li t0, 0x8000000000000000
    or t0, t0, t1
    sfence.vma
    csrw satp, t0
    sfence.vma


    // fix current address
    lla t0, 0f
    add t0, t0, s0
    jr t0
    0:
    
    lla t0, {panic}
    csrw stvec, t0

    lla sp, _stack_top
    lla      gp, __global_pointer$

    move a0, s1
    move a1, s2
    move a2, s3
    move a3, s4
    call {setup_vm}
    sfence.vma
    move a0, s1
    move a1, s2
    move a2, s3
    move a3, s4


    lla s1, {entry}
    csrw stvec, s1
    jr s1

.size _start, . - _start
.type _start, @function

    .option pop
",
    panic = sym early_panic,
    setup_vm_trampoline = sym setup_vm_trampoline,
    setup_vm = sym setup_vm,
    entry = sym kernel_entry,
    options()
);
