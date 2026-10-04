use crate::{dev::pci::{self, StatusRegister}, dtb::*, println, std::mmio::*};

#[derive(Debug)]
#[repr(C)]
struct PCICapability<T>{
    vndr: SR<u8le>,
    next: SR<u8le>,
    len: SR<u8le>,
    data: T,
}

#[derive(Debug)]
#[repr(C)]
struct VirtIOCap {
    ty: SR<u8le>,
    bar: SR<u8le>,
    id: SR<u8le>,
    padding: SR<u16le>,
    offset: SR<u32le>,
    length: SR<u32le>,
}


#[derive(Debug)]
#[repr(C)]
struct VirtIOVendorCfg {
    cfg_type: u8,
    vendor_id: u16,
}

#[derive(Debug)]
#[repr(C)]
struct VirtIOCommonCfg {
    /* About the whole device. */ 
    device_feature_select: SR<u32le>,     /* read-write */ 
    device_feature: SR<u32le>,            /* read-only for driver */ 
    driver_feature_select: SRW<u32le>,     /* read-write */ 
    driver_feature: SRW<u32le>,            /* read-write */ 
    config_msix_vector: SRW<u16le>,        /* read-write */ 
    num_queues: SR<u16le>,                /* read-only for driver */ 
    device_status: SRW<u8le>,               /* read-write */ 
    config_generation: SR<u8le>,           /* read-only for driver */ 

    /* About a specific virtqueue. */ 
    queue_select: SRW<u16le>,              /* read-write */ 
    queue_size: SRW<u16le>,                /* read-write */ 
    queue_msix_vector: SRW<u16le>,         /* read-write */ 
    queue_enable: SRW<u16le>,              /* read-write */ 
    queue_notify_off: SR<u16le>,          /* read-only for driver */ 
    queue_desc: SRW<u64le>,                /* read-write */ 
    queue_driver: SRW<u64le>,              /* read-write */ 
    queue_device: SRW<u64le>,              /* read-write */ 
    queue_notif_config_data: SR<u16le>,   /* read-only for driver */ 
    queue_reset: SRW<u16le>,               /* read-write */ 

    /* About the administration virtqueue. */ 
    admin_queue_index: SRW<u16le>,         /* read-only for driver */ 
    admin_queue_num: SRW<u16le>,         /* read-only for driver */ 
}


struct VirtIOMMIODev {
    magic: SR<u32le>,
    version: SR<u32le>,
    device_id: SR<u32le>,
    vendor_id: SR<u32le>,
    device_features: SR<u32le>,
    device_features_select: W<u32le>,
    _res0: P<[u8; 8]>,
    driver_features: W<u32le>,
    driver_features_select: W<u32le>,
    _res1: P<[u8; 8]>,
    queue_select: W<u32le>,
    queue_size_max: SR<u32le>,
    queue_size: W<u32le>,
    queue_ready: RSW<u32le>,
    queue_notify: W<u32le>,
    interrupt_status: SR<u32le>,
    interrupt_awk: W<u32le>,
    status: SRW<u32le>,
    // status: SRW<u32le>,
}

#[derive(Debug)]
#[repr(C)]
struct virtio_blk_geometry {
    cylinders: SR<u16le>,
    heads: SR<u8>,
    sectors: SR<u8>,
}

#[derive(Debug)]
#[repr(C)]
struct virtio_blk_topology { 
        // # of logical blocks per physical block (log2) 
        physical_block_exp: SR<u8le>,
        // offset of first aligned logical block 
        alignment_offset: SR<u8le>, 
        // suggested minimum I/O size in blocks 
        min_io_size: SR<u16le>,
        // optimal (suggested maximum) I/O size in blocks 
        opt_io_size: SR<u32le>,
}

#[derive(Debug)]
#[repr(C)]
struct virtio_blk_zoned_characteristics { 
        zone_sectors: SR<u32le>, 
        max_open_zones: SR<u32le>, 
        max_active_zones: SR<u32le>, 
        max_append_sectors: SR<u32le>, 
        write_granularity: SR<u32le>, 
        model: SR<u8le>,
        unused2: P<[u8; 3]>, 
}

#[derive(Debug)]
#[repr(C)]
struct virtio_blk_config { 
        capacity: SR<u64le>,
        size_max: SR<u32le>,
        seg_max: SR<u32le>,
        geometry: virtio_blk_geometry,
        blk_size: SR<u32le>,
        topology: virtio_blk_topology,
        writeback: SR<u8le>,
        unused0: P<u8le>,
        num_queues: SR<u16>,
        max_discard_sectors: SR<u32le>,
        max_discard_seg: SR<u32le>,
        discard_sector_alignment: SR<u32le>, 
        max_write_zeroes_sectors: SR<u32le>, 
        max_write_zeroes_seg: SR<u32le>, 
        write_zeroes_may_unmap: SR<u8le>, 
        unused1: P<[u8;3]>, 
        max_secure_erase_sectors: SR<u32le>, 
        max_secure_erase_seg: SR<u32le>, 
        secure_erase_sector_alignment: SR<u32le>, 
        zoned: virtio_blk_zoned_characteristics,
}

struct VirtIODev {
    cfg: *mut VirtIOCommonCfg,
    blk: *mut virtio_blk_config,
}

fn init_pci(dtb: &Dtb<'_>) -> VirtIODev {
    let Some((device, _)) = pci::pci().find_device_vendor(0x1af4, 0x1001) else {
        panic!("pci VirtIO device not found");
        // return;
    };

    let mut dev = VirtIODev {
        cfg: core::ptr::null_mut(),
        blk: core::ptr::null_mut(),
    };

    unsafe {
        let (status, command) = pci::pci().read_cmd_status(device);

        pci::pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(pci::CommandRegister::IO_SPACE, false)
                .set(pci::CommandRegister::MEMORY_SPACE, false),
        );

        println!("{status:?}");
        let capabilities = status.get(StatusRegister::CAPABILITIES_LIST);
        assert!(capabilities);
        
        let mut capabilities_ptr_off = pci::pci().pointer(device, 0x34).cast::<u8>().virt().read();
        let mut allocated = [false; 5];
        while capabilities_ptr_off != 0 {
            let cap = pci::pci()
                .pointer(device, capabilities_ptr_off as usize)
                .virt()
                .cast::<PCICapability<()>>();

            capabilities_ptr_off = (*cap).next.read().into();
            
            // might be ub here (probably)
            let data = &(*cap.cast::<PCICapability<VirtIOCap>>()).data;

            match data.ty.read().num() {
                1 => {
                    println!("{:#x?}\n{:#x?}", &*cap, data);

                    if !allocated[data.bar.read().num() as usize] {
                        pci::pci().allocate_bar(device, data.bar.read().num());
                        allocated[data.bar.read().num() as usize] = true;
                    }
                    
                    let ptr= pci::pci()
                        .read_bar(device, data.bar.read().into())
                        .pointer::<()>(pci::pci())
                        .virt()
                        .byte_add(data.offset.read().num() as usize);
                     

                    // VIRTIO_PCI_CAP_COMMON_CFG
                    dev.cfg = ptr.cast::<VirtIOCommonCfg>();
                }
                2 => {
                    // VIRTIO_PCI_CAP_NOTIFY_CFG
                    println!("VIRTIO_PCI_CAP_NOTIFY_CFG")
                }
                3 => {
                    // VIRTIO_PCI_CAP_ISR_CFG
                    println!("VIRTIO_PCI_CAP_ISR_CFG")
                }
                4 => {
                    // VIRTIO_PCI_CAP_DEVICE_CFG
                    println!("VIRTIO_PCI_CAP_DEVICE_CFG");
                    
                    if !allocated[data.bar.read().num() as usize] {
                        pci::pci().allocate_bar(device, data.bar.read().num());
                        allocated[data.bar.read().num() as usize] = true;
                    }

                    let ptr= pci::pci()
                        .read_bar(device, data.bar.read().into())
                        .pointer::<()>(pci::pci())
                        .virt()
                        .byte_add(data.offset.read().num() as usize)
                        .cast::<virtio_blk_config>();

                    println!("len: {:#x?}, bar: {:#x?}, off: {:#x?}", (*cap).len, data.bar, data.offset);
                    // println!("{:#x?}", &*ptr);
                    dev.blk = ptr;
                }
                5 => {
                    // VIRTIO_PCI_CAP_PCI_CFG
                    println!("VIRTIO_PCI_CAP_PCI_CFG")
                }
                8 => {
                    // VIRTIO_PCI_CAP_SHARED_MEMORY_CFG
                    println!("VIRTIO_PCI_CAP_SHARED_MEMORY_CFG")
                }
                9 => {
                    // VIRTIO_PCI_CAP_VENDOR_CFG
                    println!("VIRTIO_PCI_CAP_VENDOR_CFG")
                }
                _ => println!("Reserved capability type: {}", data.ty.read())
            }
        }


        pci::pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(pci::CommandRegister::IO_SPACE, true)
                .set(pci::CommandRegister::MEMORY_SPACE, true),
        );
    }

    assert!(!dev.cfg.is_null());

    dev
}

pub fn init(dtb: &Dtb<'_>) {
    println!("Initializing VirtIO");

    let dev = init_pci(dtb);

    let cfg = unsafe {&*dev.cfg};
    unsafe{
        cfg.device_status.write(0.into());
    };
    println!("{cfg:#x?}");
    println!("{:#x?}", unsafe {&*dev.blk});
    

    println!("Initialized VirtIO");
    
}
