use core::{arch::asm, fmt::Write};

use crate::{dev::uart, dtb::Dtb, println, std::stdio};

core::arch::global_asm!(
    "
.section .text.entry
.globl _start
_start:
  la sp, _stack_top
  tail {entry}
",
  entry = sym m_mode_entry,
);

#[unsafe(no_mangle)]
unsafe extern "C" fn m_mode_entry(hart_id: usize, dtb_ptr: *const u8) -> ! {
    uart::early(); // minimal for 16550: optional init

    println!("hart_id = {hart_id}");

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    _ = crate::dtb::dump(stdio::sout(), &dtb);

    crate::arch::mtrap::init(&dtb);

    unsafe {
        riscv::register::pmpcfg0::set_pmp(
            0,
            riscv::register::Range::NAPOT,
            riscv::register::Permission::RWX,
            false,
        );
        riscv::register::pmpaddr0::write(usize::MAX);

        println!("Initialized PMP");
    }

    unsafe {
        riscv::register::mideleg::set_sext();
        riscv::register::mideleg::set_ssoft();
        riscv::register::mideleg::set_stimer();

        riscv::register::mstatus::set_mpp(riscv::register::mstatus::MPP::Supervisor);
        if riscv::register::mstatus::read().mie() {
            riscv::register::mstatus::set_mpie();
        }
        println!("Initialized Supervisor registors");
    }
    unsafe {
        asm!(
          "
      csrrw x0, mepc, t0
      la sp, _stack_top
      mret
      ",
          in("t0") s_mode_entry,
          in("a0") hart_id,
          in("a1") dtb_ptr,
          options(noreturn)
        )
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn s_mode_entry(_hart_id: usize, dtb_ptr: *const u8) -> ! {
    println!("Entered S-Mode");

    use crate::dev::*;

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };

    crate::mem::init(&dtb);

    // crate::arch::strap::init(&dtb);
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
