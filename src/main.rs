#![no_std]
#![no_main]

pub mod alloc;
pub mod arch;
pub mod dev;
pub mod dtb;
pub mod fs;
pub mod interrupt;
pub mod mem;
pub mod panic;
pub mod sbi;
pub mod std;
pub mod sync;
pub mod task;
pub mod timer;
pub mod util;
pub mod syscall;

use dev::*;

use crate::{dtb::Dtb, std::stdio};


/// # Safety
/// dtb_ptr must point to a valid dtb tree
#[allow(static_mut_refs)]
pub unsafe extern "C" fn kernel_entry(
    hart_id: usize,
    dtb_ptr: *const u8,
    vma: usize,
    lma: usize,
) -> ! {

    println!("Kernel entry, hart: {hart_id}, dtb: {dtb_ptr:?}, vma: {vma:#x?}, lma: {lma:#x?}");

    unsafe{
        crate::arch::strap::init(hart_id);
    }

    unsafe {
        crate::arch::strap::begin_init_task(init_task, hart_id, dtb_ptr);
    }
}

/// # Safety
/// dtb_ptr must point to a valid dtb tree
pub unsafe extern "C" fn init_task(_hart_id: usize, dtb_ptr: *const u8) -> ! {
    println!("Begun init task");

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    println!("{dtb}");
    
    pci::init(&dtb);

    uart::init(&dtb);

    // timer::clint::init(&dtb);

    dev::test_pci::test_pci();

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

    arch::halt()
}