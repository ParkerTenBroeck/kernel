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

use core::{ffi::CStr, fmt::Write};

use crate::{dtb::Dtb, std::stdio};

fn test_pci() {
    let Some((device, _)) = pci::pci().find_device_vendor(0x1b36, 0x05) else {
        println!("test pci device not found");
        return;
    };

    unsafe {
        let (_, command) = pci::pci().read_cmd_status(device);

        pci::pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(pci::CommandRegister::IO_SPACE, false)
                .set(pci::CommandRegister::MEMORY_SPACE, false),
        );

        pci::pci().allocate_bar(device, 0);

        pci::pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(pci::CommandRegister::IO_SPACE, true)
                .set(pci::CommandRegister::MEMORY_SPACE, true),
        );

        let addr = pci::pci().read_bar(device, 0).address() as *mut ();

        for i in 0..=255 {
            addr.byte_add(0).cast::<u8>().write_volatile(i);
            addr.byte_add(1).cast::<u8>().write_volatile(4);

            let offset = addr.byte_add(4).cast::<u32>().read_volatile();
            let data = addr.byte_add(8).cast::<u32>().read_volatile();

            addr.byte_add(offset as usize)
                .cast::<u32>()
                .write_volatile(data);

            let count = addr.byte_add(12).cast::<u32>().read_volatile();
            let name = addr.byte_add(16).cast::<u8>();
            println!(
                "offset: {offset:x}, data: {data:x}, count: {count},{:?}",
                CStr::from_ptr(name)
            );
            if offset == 0 {
                break;
            }
        }
    }
}

/// # Safety
/// dtb_ptr must point to a valid dtb tree
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_main(hart_id: usize, dtb_ptr: *const u8) -> ! {
    uart::early(); // minimal for 16550: optional init

    println!("hart_id = {hart_id}");

    let dtb = unsafe { Dtb::from_ptr(dtb_ptr).unwrap() };
    _ = dtb::dump(stdio::sout(), &dtb);

    pci::init(&dtb);

    test_pci();

    uart::init(&dtb);

    vga::init(1920, 1080);
    display::update_buffer(vga::framebuffer());
    stdio::set_sout(|str| {
        uart::uart().write_str(str);
        display::print(str.as_bytes());
    });
    for c in '\x20'..='\x7E' {
        _ = stdio::sout().write_char(c)
    }
    println!();

    syscon::init(&dtb);

    arch::mtrap::init(&dtb);
    unsafe { core::arch::asm!("ecall", in("a7") 5) }

    // block::init(&dtb).unwrap();

    // syscon::poweroff();
    arch::halt()
}
