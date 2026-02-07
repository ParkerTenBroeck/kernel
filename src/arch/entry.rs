use core::fmt::Write;

use crate::{
    arch::page::{PageTable, PageTableEntry},
    dev::uart,
    dtb::Dtb,
    kernel_entry,
    mem::ROOT_PAGE,
    print,
    std::stdio,
};

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn setup_vm(_: usize, dtb_ptr: *const u8, vma: usize, pma: usize) {
    crate::stdio::set_sout(|str| {
        for byte in str.as_bytes() {
            unsafe { core::arch::asm!("ecall", in("a7") 1, in("a6") 0, in("a0") *byte) }
        }
    });
    stdio::sout().write_str("Setting up virtual memory\n");

    for i in 0..256 {
        ROOT_PAGE.entries[i] = (PageTableEntry::COM_DEV.set_executable(true)
            | PageTableEntry::DIRTY_ACCESSED)
            .set_ppn((i as u64) << 18);
    }

    uart::uart().write_str("Discovering memory\n");

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

        crate::alloc::buddy::BUDDY
            .lock()
            .free_region(curr_phys_addr as *mut u8, page_size);

        uart::uart().write_str("freeing start: ");
        uart::uart().hex(curr_phys_addr);
        uart::uart().write_str(" end: ");
        uart::uart().hex(curr_phys_addr + page_size);
        uart::uart().write_str("\n");
        curr_phys_addr += page_size;
    }

    uart::uart().write_str("Memory discovery complete\n");

    uart::uart().write_str("Mapping kernel memory\n");

    uart::uart().write_str("pma: ");
    uart::uart().hex(pma);
    uart::uart().write_str("\n");

    uart::uart().write_str("vma: ");
    uart::uart().hex(vma);
    uart::uart().write_str("\n");

    let virt_offset = vma - pma;

    uart::uart().write_str("offset: ");
    uart::uart().hex(virt_offset);
    uart::uart().write_str("\n");

    // text section
    crate::mem::map_pages(
        kernel_layout_phys.text.start + virt_offset,
        kernel_layout_phys.text.start,
        kernel_layout_phys.text.end - kernel_layout_phys.text.start,
        PageTableEntry::COM_EXEC | PageTableEntry::DIRTY_ACCESSED,
    );

    // ro section
    crate::mem::map_pages(
        kernel_layout_phys.ro_data.start + virt_offset,
        kernel_layout_phys.ro_data.start,
        kernel_layout_phys.ro_data.end - kernel_layout_phys.ro_data.start,
        PageTableEntry::COM_RO | PageTableEntry::DIRTY_ACCESSED,
    );

    // data section
    crate::mem::map_pages(
        kernel_layout_phys.data.start + virt_offset,
        kernel_layout_phys.data.start,
        kernel_layout_phys.data.end - kernel_layout_phys.data.start,
        PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
    );

    // bss section
    crate::mem::map_pages(
        kernel_layout_phys.bss.start + virt_offset,
        kernel_layout_phys.bss.start,
        kernel_layout_phys.bss.end - kernel_layout_phys.bss.start,
        PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
    );

    // stacks
    crate::mem::map_pages(
        kernel_layout_phys.stack.start + virt_offset,
        kernel_layout_phys.stack.start,
        kernel_layout_phys.stack.end - kernel_layout_phys.stack.start,
        PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
    );

    _ = PageTable::disp_table(&raw const ROOT_PAGE, 0, 0, uart::uart());

    uart::uart().write_str("Completed kernel mapping\n");
}

core::arch::global_asm!(
    "
.section .text.entry
.global _start
_start:

    // clear bss
    la a3, _bss_start
    la a4, _bss_end
    ble a4, a3, .Lclear_bss_done
    .Lclear_bss:
        ld zero, 0(a3)
        add a3, a3, 8
        blt a3, a4, .Lclear_bss
    .Lclear_bss_done:

    
    lga a2, KERNEL_VMA
    la a3, _kernel_start
    // fixup offset
    sub s0, a2, a3

    la sp, _stack_top
    move s1, a0
    move s2, a1
    move s3, a2
    move s4, a3
    call {setup_vm}
    move a0, s1
    move a1, s2
    move a2, s3
    move a3, s4

    // fixed stack pointer
    la sp, _stack_top
    add sp, sp, s0

    // fixed entry address
    la s1, {entry}
    add s1, s1, s0

    // sets trap vector to entry point
    csrw stvec, s1

    // setup virtual memory
    la t1, {root}
    srli t1, t1, 12
    li t0, 0x8000000000000000
    or t0, t0, t1
    sfence.vma
    csrw satp, t0
    sfence.vma

    jr s1

.size _start, . - _start
.type _start, @function
",
    setup_vm = sym setup_vm,
    root = sym ROOT_PAGE,
    entry = sym kernel_entry,
    options()
);
