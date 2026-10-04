use crate::{dev::pci, dtb::Dtb, println, std::mmio::*};

#[derive(Debug)]
#[repr(C)]
struct RTL8139 {
    // 0x00
    // IDR0..IDR5
    //
    // Datasheet says writes are only permitted with 32-bit accesses.
    // Representing this as raw bytes preserves the exact register layout.
    mac0: SR<u8>,
    mac1: SR<u8>,
    mac2: SR<u8>,
    mac3: SR<u8>,
    mac4: SR<u8>,
    mac5: SR<u8>,

    // 0x06
    _reserved0: P<[u8; 2]>,

    // 0x08
    // MAR0..MAR7
    mar: SR<u64le>,

    // 0x10
    // TSD0..TSD3
    tsd: RW<[u32le; 4]>,

    // 0x20
    // TSAD0..TSAD3
    tsad: RW<[u32le; 4]>,

    // 0x30
    // Receive buffer start address
    rbstart: RW<u32le>,

    // 0x34
    // Early receive byte count
    erbcr: R<u16le>,

    // 0x36
    // Early receive status register
    ersr: R<u8le>,

    // 0x37
    // Command register
    cmd: RW<u8le>,

    // 0x38
    // Current address of packet read
    capr: RW<u16le>,

    // 0x3A
    // Current buffer address
    cbr: R<u16le>,

    // 0x3C
    // Interrupt mask register
    imr: RW<u16le>,

    // 0x3E
    // Interrupt status register
    isr: RW<u16le>,

    // 0x40
    // Transmit configuration register
    tcr: RW<u32le>,

    // 0x44
    // Receive configuration register
    rcr: RW<u32le>,

    // 0x48
    // Timer count register
    tctr: RW<u32le>,

    // 0x4C
    // Missed packet counter
    //
    // Only the low 24 bits are valid.
    mpc: RW<u32le>,

    // 0x50
    // 93C46 / 93C56 command register
    eeprom_cmd: RW<u8le>,

    // 0x51
    config0: RW<u8le>,

    // 0x52
    config1: RW<u8le>,

    // 0x53
    _reserved1: P<u8>,

    // 0x54
    // Timer interrupt register
    timer_int: RW<u32le>,

    // 0x58
    // Media status register
    msr: RW<u8le>,

    // 0x59
    config3: RW<u8le>,

    // 0x5A
    config4: RW<u8le>,

    // 0x5B
    _reserved2: P<u8>,

    // 0x5C
    // Multiple interrupt select
    mulint: RW<u16le>,

    // 0x5E
    // PCI revision ID
    rerid: R<u8le>,

    // 0x5F
    _reserved3: P<u8>,

    // 0x60
    // Transmit status of all descriptors
    tsad_status: R<u16le>,

    // 0x62
    // Basic mode control register
    bmcr: RW<u16le>,

    // 0x64
    // Basic mode status register
    bmsr: R<u16le>,

    // 0x66
    // Auto-negotiation advertisement register
    anar: RW<u16le>,

    // 0x68
    // Auto-negotiation link partner register
    anlpar: R<u16le>,

    // 0x6A
    // Auto-negotiation expansion register
    aner: R<u16le>,

    // 0x6C
    // Disconnect counter
    dis: R<u16le>,

    // 0x6E
    // False carrier sense counter
    fcsc: R<u16le>,

    // 0x70
    // N-way test register
    nwaytr: RW<u16le>,

    // 0x72
    // RX_ER counter
    rec: R<u16le>,

    // 0x74
    // CS configuration register
    cscr: RW<u16le>,

    // 0x76
    _reserved4: P<[u8; 2]>,

    // 0x78
    // PHY parameter 1
    phy1_parm: RW<u32le>,

    // 0x7C
    // Twister parameter
    tw_parm: RW<u32le>,

    // 0x80
    // PHY parameter 2
    phy2_parm: RW<u8le>,

    // 0x81
    _reserved5: P<[u8; 3]>,

    // 0x84
    // Power management CRC registers
    crc: RW<[u8; 8]>,

    // 0x8C
    // Wakeup frame 0..7
    wakeup: RW<[u64le; 8]>,

    // 0xCC
    // LSB mask byte / CRC for wakeup frame 0..7
    lsbcrc: RW<[u8; 8]>,

    // 0xD4
    // Flash memory read/write register
    flash: RW<u32le>,

    // 0xD8
    config5: RW<u8le>,

    // 0xD9
    _reserved6: P<[u8; 0x17]>,

    // 0xF0
    // CardBus only
    fer: RW<u32le>,

    // 0xF4
    // CardBus only
    femr: RW<u32le>,

    // 0xF8
    // CardBus only
    fpsr: R<u32le>,

    // 0xFC
    // CardBus only
    ffer: W<u32le>,
}

fn init_pci(dtb: &Dtb<'_>) -> &'static RTL8139 {
    let Some((device, _)) = pci::pci().find_device_vendor(0x10ec, 0x8139) else {
        panic!("pci RTL8139 device not found");
        // return;
    };

    let dev = unsafe {
        let (status, command) = pci::pci().read_cmd_status(device);

        pci::pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(pci::CommandRegister::IO_SPACE, false)
                .set(pci::CommandRegister::BUS_MASTER, false)
                .set(pci::CommandRegister::MEMORY_SPACE, false),
        );

        pci::pci().allocate_bar(device, 0);

        pci::pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(pci::CommandRegister::IO_SPACE, true)
                .set(pci::CommandRegister::BUS_MASTER, true)
                .set(pci::CommandRegister::MEMORY_SPACE, true),
        );

        pci::pci().read_bar(device, 0).pointer::<RTL8139>(pci::pci()).virt()
    };

    println!("{:#x?}", unsafe{&*dev});

    unsafe {&*dev}
}

pub fn init(dtb: &Dtb<'_>) {

    println!("Initializing RTL8139");

    let dev = init_pci(dtb);

    println!("Initizlied RTL8139 PCI");

    unsafe {
        dev.config0.write(0x0.into());

        dev.cmd.write(0x10.into());
        while dev.cmd.read().num() & 0x10 != 0 {}

        // crate::mem::pages::BUDDY.lock().alloc(layout)

    }
    println!("Initizlied RTL8139 Dev");
    
    println!("Initizlied RTL8139");
}