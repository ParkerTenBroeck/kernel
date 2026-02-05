
use crate::{arch::page::{PageTable, PageTableEntry}, dev::uart, dtb::{self, ByteStream, Dtb, DtbProperties}, println, std::stdio};


pub static mut ROOT_PAGE: PageTable = PageTable{
        entries: [PageTableEntry::new(); 512],
};

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "C" fn setup_vm(_: usize, dtb_ptr: *const u8, vma: usize, lma: usize){
    uart::early();
    stdio::set_sout(|str|{
        uart::uart().write_str(str);
    });

    for i in 0..256{
        ROOT_PAGE.entries[i] = PageTableEntry::new()
            .set_readable(true)
            .set_writable(true)
            .set_executable(true)
            .set_valid(true)
            .set_accessed(true)
            .set_dirty(true) 
            .set_ppn((i as u64) << 18);
    }

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    
    let node = dtb.nodes().find(|node| {
        node.properties()
            .find(b"device_type")
            .is_some_and(|v| v.contains_str(b"memory"))
    }).expect("cannot find 'memory' device");

    let addr_cells = dtb.root().properties().expect_value(b"#address-cells", ByteStream::u32)*4;
    let size_cells = dtb.root().properties().expect_value(b"#size-cells", ByteStream::u32)*4;
    let reg_cells = [addr_cells, size_cells];

    let [start, size] = node.properties().expect_value(b"reg", |stream|stream.usize_cells_arr(reg_cells));

     uart::uart().write_str("\nmode ");
    print_hex_u64(start as u64);
}


core::arch::global_asm!(
    "
.section .text.entry
.global _start
_start:

    // clear bss
    /*
    la a3, _bss_start
    la a4, _bss_end
    ble a4, a3, .Lclear_bss_done
    .Lclear_bss:
        ld zero, 0(a3)
        add a3, a3, 8
        blt a3, a4, .Lclear_bss
    .Lclear_bss_done:
    */

    
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

    // fixed device tree pointer
    add a0, a0, s0

    // fixed stack pointer
    la sp, _stack_top
    add sp, sp, s0

    // fixed entry address
    la s1, {entry}
    add s1, s1, s0

    // sets trap vector to entry point
    csrw stvec, s1

    // calculate page table entry for kernel start
    srli t2, a3, 1
    ori t2, t2, 0b1111

    la t1, {root}
    //sd t2, 511(t1)

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
    entry = sym s_mode_entry,
    options()
);

#[inline]
pub fn print_hex_u64(value: u64) {
    crate::uart::uart().write_bytes(b"0x");

    // Print exactly 16 hex digits (leading zeros included)
    for i in (0..16).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;
        let c = match nibble {
            0..=9  => b'0' + nibble,
            10..=15 => b'a' + (nibble - 10),
            _ => unreachable!(),
        };
        crate::uart::uart().write_bytes(&[c]);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn s_mode_entry(hart_id: usize, dtb_ptr: *const u8, vma: usize, lma: usize) -> ! {
    uart::early();

    // match riscv::register::satp::read().mode(){
    //     riscv::register::satp::Mode::Bare => uart::uart().write_str("Bare"),
    //     riscv::register::satp::Mode::Sv39 => uart::uart().write_str("Sv39"),
    //     riscv::register::satp::Mode::Sv48 => uart::uart().write_str("Sv48"),
    //     riscv::register::satp::Mode::Sv57 => uart::uart().write_str("Sv57"),
    //     riscv::register::satp::Mode::Sv64 => uart::uart().write_str("Sv64"),
    // }
     uart::uart().write_str("\nmode ");
    print_hex_u64(ROOT_PAGE.entries[256].ppn());
    uart::uart().write_str("\nvma ");
    print_hex_u64(vma as u64);
    uart::uart().write_str("\nlma ");
    print_hex_u64(lma as u64);
    (0xFFFFFFC001000000 as *const u32).read_volatile();
    uart::uart().write_str("\nvma ");
    loop{}
    // writeln!(uart::uart(), "Entered S-Mode, hart: {hart_id}, vma: {vma:x?}, lma: {lma:x?}");

    use crate::dev::*;

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    _ = dtb::dump(stdio::sout(), &dtb);

    crate::arch::strap::init(&dtb);

    crate::mem::init(&dtb);

    // //     println!("hart_id = {hart_id}");

    // //     let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    // //     _ = dtb::dump(stdio::sout(), &dtb);

    // pci::init(&dtb).unwrap();

    // //     test_pci();

    // //     uart::init(&dtb).unwrap();

    // vga::init(1920, 1080);
    // display::update_buffer(vga::framebuffer());
    // stdio::set_sout(|str| {
    //     uart::uart().write_str(str);
    //     display::print(str.as_bytes());
    // });
    // for c in '\x20'..='\x7E' {
    //     _ = stdio::sout().write_char(c)
    // }
    // println!();

    crate::dev::syscon::init(&dtb);

    // //     arch::mtrap::init(&dtb);
    // //     unsafe{
    // //         core::arch::asm!("ecall", in("a7") 5)
    // //     }

    // //     // block::init(&dtb).unwrap();

    // #[allow(static_mut_refs)]
    // unsafe {
    //     let base = 0x80000000usize + 0x8000000 / 2;
    //     crate::alloc::buddy::BUDDY.lock().free_order((base + 2 * (1 << 12)) as *mut u8, 1, 0);
    //     crate::alloc::buddy::BUDDY.lock().print();
    //     println!();
    //     crate::alloc::buddy::BUDDY.lock().free_order((base) as *mut u8, 1, 0);
    //     crate::alloc::buddy::BUDDY.lock().print();
    //     println!();
    //     crate::alloc::buddy::BUDDY.lock().free_order((base + 3 * (1 << 12)) as *mut u8, 1, 0);
    //     crate::alloc::buddy::BUDDY.lock().print();
    //     println!();
    //     crate::alloc::buddy::BUDDY.lock().free_order((base + (1 << 12)) as *mut u8, 1, 0);
    //     crate::alloc::buddy::BUDDY.lock().print();
    // }

    crate::dev::syscon::poweroff();
}
