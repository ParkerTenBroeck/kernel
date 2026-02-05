#![no_std]
#![no_main]

pub mod alloc;
pub mod arch;
pub mod dev;
pub mod dtb;
pub mod fs;
pub mod mem;
pub mod panic;
pub mod std;
pub mod sync;
pub mod util;

use dev::*;


use crate::{dtb::Dtb, std::stdio};

/// # Safety
/// dtb_ptr must point to a valid dtb tree
pub unsafe extern "C" fn kernel_entry(hart_id: usize, dtb_ptr: *const u8, vma: usize, lma: usize) -> ! {
    uart::early(); // minimal for 16550: optional init
    crate::arch::strap::init();

    println!("Entered S-Mode, hart: {hart_id}, dtb: {dtb_ptr:?}, vma: {vma:#x?}, lma: {lma:#x?}");
   

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    _ = dtb::dump(stdio::sout(), &dtb);

    pci::init(&dtb);
    
    dev::test_pci::test_pci();
    
    uart::init(&dtb);

    vga::init(1920, 1080);
    display::update_buffer(vga::framebuffer());

    stdio::set_sout(|str| {
        uart::uart().write_str(str);
        display::print(str.as_bytes());
    });

    for c in '\x20'..='\x7E' {
        use core::fmt::Write;
        _ = stdio::sout().write_char(c)
    }
    println!();

    syscon::init(&dtb);

    
    unsafe { core::arch::asm!("ecall", in("a7") 5) }
    arch::halt()
}
