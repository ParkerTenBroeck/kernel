use core::fmt::{self, Write};
use core::ops::Add;
use core::ptr::{addr_of_mut, read_volatile, write_volatile};

use crate::{dtb::*, pci, println};

// 16550 registers (offsets)
const RBR_THR_DLL: usize = 0x00; // receive / transmit / divisor low
const IER_DLM: usize = 0x01; // interrupt enable / divisor high
const LCR: usize = 0x03; // line control
const LSR: usize = 0x05; // line status

const LSR_TX_IDLE: u8 = 1 << 5; // THR empty

#[repr(C)]
struct NS16550A{
    rbr_thr_dll: u8,
    ier_dlm: u8,
    pad0: u8,
    lcr: u8,
    pad1: u8,
    lsr: u8,
}

static mut UART: *mut NS16550A = 0x1000_0000 as *mut NS16550A;

pub fn early(){
    basic_init();
}

pub fn init(dtb: &Dtb) -> Result<(), DtbError>{

    println!("initializing UART");

    let node = dtb.find_compatable_nodes(b"ns16550a")?.expect_one()?;

    let props = node.properties()?;
    let interrupts = props.expect(b"interrupts")?.u32()?;
    let interrupts_parent = props.expect(b"interrupt-parent")?.u32()?;
    let clock_frequency = props.expect(b"clock-frequency")?.u32()?;
    let [start, size] = props.expect(b"reg")?.u64_array::<2>()?;

    println!("{start:#x?}");

    // unsafe{
    //     let Some((device, _)) = pci::pci().find_device_vendor(0x1b36, 0x02) else {
    //         panic!("uart pci device not found")
    //     };

    //     let (_, command) = pci::pci().read_cmd_status(device);

    //     pci::pci().write_cmd_status(device, 
    //         *command.clone()
    //         .set(pci::CommandRegister::IO_SPACE, false)
    //         .set(pci::CommandRegister::MEMORY_SPACE, false)
    //     );

    //     pci::pci().allocate_bar(device, 0);

    //     pci::pci().write_cmd_status(device, 
    //         *command.clone()
    //         .set(pci::CommandRegister::IO_SPACE, true)
    //         .set(pci::CommandRegister::MEMORY_SPACE, true)
    //     );

    //     let start = pci::pci().read_bar(device, 0).address();


    //     let mut uart = d_16550::Uart16550::new_with_stride(start, 1);
    //     uart.init(1);
    //     _ = uart.write_str("Uart Hello\n");
    // }

    unsafe{
        UART = 0x1000_0000 as *mut NS16550A;
        basic_init();
    }
    Ok(())
}

fn basic_init(){
    unsafe{
        // 8n1 (8 bits, no parity, 1 stop)
        addr_of_mut!((*UART).lcr).write_volatile(0x03); 
        // Disable interrupts
        addr_of_mut!((*UART).ier_dlm).write_volatile(0x00); 
    }
}

pub fn putc(c: u8) {
    unsafe {
        // Wait until THR empty
        while (addr_of_mut!((*UART).lsr).read_volatile() & LSR_TX_IDLE) == 0 {}
        addr_of_mut!((*UART).rbr_thr_dll).write_volatile(c);
    }
}

pub fn puts(s: &str) {
    for b in s.bytes() {
        if b == b'\n' {
            putc(b'\r');
        }
        putc(b);
    }
}

pub fn putb(b: &[u8]){
    for b in b{
        putc(*b)
    }
}

pub struct UartWriter;
impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        puts(s);
        Ok(())
    }
}



