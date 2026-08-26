use crate::{dev::pci::{self, StatusRegister}, dtb::*, pci::pci, println};

#[derive(Debug)]
#[repr(C)]
struct VirtIoPciCap {
    cap_vndr: u8,
    cap_next: u8,
    cap_len: u8,
    cap_type: u8,
    bar: u8,
    id: u8,
    padding: [u8;2],
    offset: u32,
    length: u32,
}

pub fn init(dtb: &Dtb<'_>) {
    println!("Initializing VirtIO");

    let Some((device, _)) = pci::pci().find_device_vendor(0x1af4, 0x1001) else {
        println!("pci VirtIO device not found");
        return;
    };

    unsafe {
        let (status, command) = pci::pci().read_cmd_status(device);

        println!("{status:?}");
        let capabilities = status.get(StatusRegister::CAPABILITIES_LIST);
        assert!(capabilities);
        
        let mut capabilities_ptr_off = pci::pci().pointer(device, 0x34).cast::<u8>().virt().read();
        while capabilities_ptr_off != 0 {
            let cap = &*pci::pci().pointer(device, capabilities_ptr_off as usize).virt().cast::<VirtIoPciCap>();
            println!("{cap:#?}");
            capabilities_ptr_off = cap.cap_next;
        }

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

        let addr = pci::pci()
            .read_bar(device, 0)
            .pointer::<()>(pci::pci())
            .virt();
    }
    println!("Initialized VirtIO");
    
}
