
use crate::{dtb::{self, Dtb}, println, std::stdio};

core::arch::global_asm!(
    "
.section .text.entry
.globl _start
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

    la sp, _stack_top

    
    la a2, KERNEL_VMA
    la a3, KERNEL_LMA
    tail {entry}
",
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

    uart::uart().write_str("\nvma ");
    print_hex_u64(vma as u64);
    uart::uart().write_str("\nlma ");
    print_hex_u64(lma as u64);
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
