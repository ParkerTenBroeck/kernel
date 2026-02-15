use core::ffi::CStr;

use crate::{dev::pci, println};

pub fn test_pci() {
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

        let addr = pci::pci().read_bar(device, 0).pointer::<()>(pci::pci()).virt();

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
