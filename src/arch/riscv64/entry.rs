use core::{
    arch::asm,
    fmt::Write,
    sync::atomic::{AtomicBool, fence},
};

use crate::{
    arch::page::{PageTable, PageTableEntry, PageTableRoot},
    dev::uart,
    dtb::Dtb,
    kernel_entry,
    mem::{KernelLayout},
    println,
    std::stdio,
};

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn m_mode_setup(_: usize, _: *const u8, _vma: usize, pma: usize) {
    uart::early_pre_vm();
    crate::stdio::set_sout(early_print);
    relocate_kernel(pma);

    riscv::register::mtvec::write(riscv::register::mtvec::Mtvec::from_bits(early_panic as *mut() as usize));

    println!("Entered M-Mode");

    let mut medeleg = riscv::register::medeleg::read();
    medeleg.set_breakpoint(true);
    medeleg.set_illegal_instruction(true);
    medeleg.set_instruction_fault(true);
    medeleg.set_instruction_misaligned(true);
    medeleg.set_instruction_page_fault(true);
    medeleg.set_load_fault(true);
    medeleg.set_load_misaligned(true);
    medeleg.set_store_fault(true);
    medeleg.set_store_page_fault(true);
    medeleg.set_supervisor_env_call(true);
    medeleg.set_user_env_call(true);
    riscv::register::medeleg::write(medeleg);

    let mut mideleg = riscv::register::mideleg::read();
    mideleg.set_sext(true);
    mideleg.set_ssoft(true);
    mideleg.set_stimer(true);
    riscv::register::mideleg::write(mideleg);

    riscv::register::pmpcfg0::set_pmp(
        0,
        riscv::register::Range::OFF,
        riscv::register::Permission::RWX,
        false,
    );
    riscv::register::pmpcfg0::set_pmp(
        1,
        riscv::register::Range::TOR,
        riscv::register::Permission::RWX,
        false,
    );
    riscv::register::pmpaddr0::write(0);
    riscv::register::pmpaddr1::write(usize::MAX);

    riscv::register::mie::clear_mext();
    riscv::register::mie::clear_msoft();
    riscv::register::mie::clear_mtimer();

    riscv::register::mie::set_sext();
    riscv::register::mie::set_ssoft();
    riscv::register::mie::set_stimer();

    let mut mstatus = riscv::register::mstatus::read();
    mstatus.set_mpie(false);
    mstatus.set_mpp(riscv::register::mstatus::MPP::Supervisor);
    riscv::register::mstatus::write(mstatus);

    riscv::register::mcounteren::set_cy();
    for i in 0..64 {
        _ = riscv::register::mcounteren::try_set_hpm(i);
    }
    riscv::register::mcounteren::set_ir();
    riscv::register::mcounteren::set_tm();

    riscv::register::mtvec::write(riscv::register::mtvec::Mtvec::new(
        early_panic as *mut () as usize,
        riscv::register::stvec::TrapMode::Direct,
    ));

    println!("Configured M-Mode");

    asm!(
        "
        lla t0, 0f
        csrw mepc, t0
        mret
        0:
    ")
}

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
unsafe fn relocate_kernel(addr: usize) {
    _ = stdio::sout().write_str("relocating kernel\n");
    unsafe {
        super::reloc::relocate_kernel(addr);
    }

    fence(core::sync::atomic::Ordering::SeqCst);

    crate::stdio::set_sout(early_print);

    println!("relocated kernel to {addr:#x?}");
}

pub fn early_print(str: &str) {
    uart::uart().write_str(str);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn setup_vm_trampoline(
    _: usize,
    _: *const u8,
    vma: usize,
    pma: usize,
) -> *const PageTable {
    if !FIRST.swap(false, core::sync::atomic::Ordering::SeqCst) {
        early_print("\n\nNOT FIRST");
        early_panic();
    }
    crate::stdio::set_sout(early_print);
    relocate_kernel(pma);


    assert!(pma.is_multiple_of(1 << (12 + 9)), "pma not properly aligned {pma:#x?}");
    assert!(vma.is_multiple_of(1 << (12 + 9)), "vma not properly aligned {vma:#x?}");

    println!("Physical kernel layout: {:#x?}", KernelLayout::new());

    println!("Setting up virtual memory trampoline");

    for i in 0..256 {
        TRAMPOLINE_ROOT_PAGE.entries[i] = (PageTableEntry::COM_DEV.set_executable(true)
            | PageTableEntry::DIRTY_ACCESSED)
            .set_ppn((i as u64) << 18);
    }

    for i in 0..128 {
        TRAMPOLINE_ROOT_PAGE.entries[i + 256] =
            (PageTableEntry::COM_DEV | PageTableEntry::DIRTY_ACCESSED).set_ppn((i as u64) << 18);
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
            .set_ppn((pma as u64 >> 12) + ((i as u64) << 9) + (512 << 9))
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

    println!("{dtb}");

    crate::mem::pages::discover(&dtb, vma, pma);

    println!("Creating kernel memory map");

    let phys_offset = pma.wrapping_sub(vma);

    println!("pma: {pma:#x?}    vma: {vma:#x?}    offset: {phys_offset:#x?}");

    let kernel_layout = KernelLayout::new();

    let supplier = || unsafe { crate::mem::pages::pages_zeroed(1).cast() };

    let mut kernel_map = PageTableRoot::new(supplier);

    // virt <-> phys
    kernel_map
        .map_phys_region(
            crate::mem::PHYS_ADDR_OFFSET,
            0x0,
            1024 * 1024 * 1024 * 128,
            PageTableEntry::COM_DEV | PageTableEntry::DIRTY_ACCESSED,
            supplier,
        )
        .unwrap();

    // text section
    kernel_map
        .map_phys_region(
            kernel_layout.text.start,
            kernel_layout.text.start.wrapping_add(phys_offset),
            kernel_layout.text.end - kernel_layout.text.start,
            PageTableEntry::COM_EXEC | PageTableEntry::DIRTY_ACCESSED,
            supplier,
        )
        .unwrap();

    // ro section
    kernel_map
        .map_phys_region(
            kernel_layout.ro_data.start,
            kernel_layout.ro_data.start.wrapping_add(phys_offset),
            kernel_layout.ro_data.end - kernel_layout.ro_data.start,
            PageTableEntry::COM_RO | PageTableEntry::DIRTY_ACCESSED,
            supplier,
        )
        .unwrap();

    // data section
    kernel_map
        .map_phys_region(
            kernel_layout.data.start,
            kernel_layout.data.start.wrapping_add(phys_offset),
            kernel_layout.data.end - kernel_layout.data.start,
            PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
            supplier,
        )
        .unwrap();

    // bss section
    kernel_map
        .map_phys_region(
            kernel_layout.bss.start,
            kernel_layout.bss.start.wrapping_add(phys_offset),
            kernel_layout.bss.end - kernel_layout.bss.start,
            PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
            supplier,
        )
        .unwrap();

    // stacks
    kernel_map
        .map_phys_region(
            kernel_layout.stack.start,
            kernel_layout.stack.start.wrapping_add(phys_offset),
            kernel_layout.stack.end - kernel_layout.stack.start,
            PageTableEntry::COM_RW | PageTableEntry::DIRTY_ACCESSED,
            supplier,
        )
        .unwrap();

    println!("{kernel_map}");

    asm!("sfence.vma");
    riscv::register::satp::set(
        riscv::register::satp::Mode::Sv39,
        0,
        kernel_map.root().phys() as usize >> 12,
    );
    asm!("sfence.vma");

    {

    }

    uart::early_post_vm();

    // crate::mem::pages::free_page(Pointer::from_virt(&raw mut TRAMPOLINE_ROOT_PAGE).cast());
    // crate::mem::pages::free_page(Pointer::from_virt(&raw mut TRAMPOLINE_ROOT_PAGE_L2_LOW).cast());
    // crate::mem::pages::free_page(Pointer::from_virt(&raw mut TRAMPOLINE_ROOT_PAGE_L2_HIGH).cast());

    crate::alloc::init();

    println!("Completed kernel meory map");
}

unsafe extern "C" fn early_panic() {
    uart::early_pre_vm();
    early_print("\n\nearly panic\n\n");

    uart::uart()
        .str("sepc: ")
        .hex(riscv::register::sepc::read())
        .str("\nstval: ")
        .hex(riscv::register::stval::read())
        .str("\nscause: ")
        .hex(riscv::register::scause::read().bits());
    super::halt()
}

core::arch::global_asm!(
    "
        .option push
        .option norelax
.section .text.entry
.global _start
_start:

    // clear bss
    lla a3, _bss_start
    lla a4, _bss_end
    ble a4, a3, .Lclear_bss_done
    .Lclear_bss:
        ld zero, 0(a3)
        add a3, a3, 8
        blt a3, a4, .Lclear_bss
    .Lclear_bss_done:

    
    lga a2, KERNEL_LINK_ADDR
    lla a3, _kernel_start
    // fixup offset
    sub s0, a2, a3

    lla sp, _stack_top

    move s1, a0
    move s2, a1
    move s3, a2
    move s4, a3

    // enables supervisor software interrupts
    li t0, 2
    csrs sie, t0
    csrs sstatus, t0

    // lla t0, .LmModeDone
    // csrw stvec, t0
        
    //     lla t0, {panic}
    //     csrw mtvec, t0
        
    //     call {m_mode_setup}
    //     move a0, s1
    //     move a1, s2
    //     move a2, s3
    //     move a3, s4   
    // .LmModeDone:

    lla t0, {panic}
    csrw stvec, t0

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

    // fix device tree pointer
    li t0, {phys_to_virt_offset}
    add s2, s2, t0

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
    m_mode_setup = sym m_mode_setup,
    setup_vm_trampoline = sym setup_vm_trampoline,
    setup_vm = sym setup_vm,
    entry = sym kernel_entry,
    phys_to_virt_offset = const crate::mem::PHYS_ADDR_OFFSET,
    options()
);
