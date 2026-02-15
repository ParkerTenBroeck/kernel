use core::{arch::asm, fmt::Write, sync::atomic::{AtomicBool, fence}};

use crate::{
    arch::page::{PageTable, PageTableEntry, PageTableRoot}, dev::uart, dtb::Dtb, kernel_entry, mem::KernelLayout, println, std::stdio
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

pub fn early_print(str: &str){
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


    println!("Discovering memory: {dtb_ptr:?}");

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };

    crate::mem::pages::discover(&dtb);

    println!("Creating kernel memory map");

    let phys_offset = pma.wrapping_sub(vma);

    println!("pma: {pma:#x?}    vma: {vma:#x?}    offset: {phys_offset:#x?}");

    let kernel_layout = KernelLayout::new();

    let supplier = || unsafe{crate::mem::pages::page_zeroed()};

    let mut kernel_map = PageTableRoot::new(supplier);

    // text section
    kernel_map.map_region(
        kernel_layout.text.start,
        kernel_layout.text.start.wrapping_add(phys_offset),
        kernel_layout.text.end - kernel_layout.text.start,
        PageTableEntry::COM_EXEC | PageTableEntry::DIRTY_ACCESSED,
        supplier
    ).unwrap();

    // ro section
    kernel_map.map_region(
        kernel_layout.ro_data.start,
        kernel_layout.ro_data.start.wrapping_add(phys_offset),
        kernel_layout.ro_data.end - kernel_layout.ro_data.start,
        PageTableEntry::COM_RO | PageTableEntry::DIRTY_ACCESSED,
        supplier
    ).unwrap();

    // data section
    kernel_map.map_region(
        kernel_layout.data.start,
        kernel_layout.data.start.wrapping_add(phys_offset),
        kernel_layout.data.end - kernel_layout.data.start,
        PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
        supplier
    ).unwrap();

    // bss section
    kernel_map.map_region(
        kernel_layout.bss.start,
        kernel_layout.bss.start.wrapping_add(phys_offset),
        kernel_layout.bss.end - kernel_layout.bss.start,
        PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
        supplier
    ).unwrap();

    // stacks
    kernel_map.map_region(
        kernel_layout.stack.start,
        kernel_layout.stack.start.wrapping_add(phys_offset),
        kernel_layout.stack.end - kernel_layout.stack.start,
        PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
        supplier
    ).unwrap();

    // virt <-> phys
    kernel_map.map_region(
        crate::mem::PHYS_ADDR_OFFSET,
        0x0,
        1024*1024*1024*128,
        PageTableEntry::COM_DEV | PageTableEntry::DIRTY_ACCESSED,
        supplier
    ).unwrap();

    println!("{kernel_map}");

    asm!("sfence.vma");
    riscv::register::satp::set(riscv::register::satp::Mode::Sv39, 0, kernel_map.root().phys() as usize >> 12);
    asm!("sfence.vma");

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

    li t0, {phys_to_virt_offset}
    add s2, s2, t0

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
    phys_to_virt_offset = const crate::mem::PHYS_ADDR_OFFSET,
    options()
);
