use crate::{dtb::*, mem::Pointer, println, stdio};

static mut UART: Uart16550 = unsafe { Uart16550::new(0x1000_0000 as *mut ()) };

pub fn uart() -> &'static mut Uart16550 {
    unsafe { (&raw mut UART).as_mut().unwrap_unchecked() }
}

pub fn early() {
    uart().init(1);
    stdio::set_sout(|str| uart().write_str(str));
}

pub fn init(dtb: &Dtb) {
    println!("initializing UART");

    let node = dtb
        .nodes()
        .compatible(b"ns16550a")
        .next()
        .expect("cannot find ns16550a compatable device");

    let props = node.properties();
    let _interrupts = props.expect_value(b"interrupts", ByteStream::u32);
    let _interrupts_parent = props.expect_value(b"interrupt-parent", ByteStream::u32);
    let clock_frequency = props.expect_value(b"clock-frequency", ByteStream::u32);
    let [start, _size] = props.expect_value(b"reg", ByteStream::u64_array::<2>);

    unsafe {
        UART = Uart16550::new(Pointer::from_phys(start as *mut ()).virt());
        uart().init((clock_frequency / (16 * 115200)) as u16);
    }

    println!("Initialized UART");

    stdio::set_sout(|str| uart().write_str(str));

    unsafe{
        use crate::dev::pci;

        let Some((device, _)) = pci::pci().find_device_vendor(0x1b36, 0x02) else {
            panic!("uart pci device not found")
        };

        let (_, command) = pci::pci().read_cmd_status(device);

        pci::pci().write_cmd_status(device,
            *command.clone()
            .set(pci::CommandRegister::IO_SPACE, false)
            .set(pci::CommandRegister::MEMORY_SPACE, false)
        );

        pci::pci().allocate_bar(device, 0);

        pci::pci().write_cmd_status(device,
            *command.clone()
            .set(pci::CommandRegister::IO_SPACE, true)
        );

        let start = pci::pci().read_bar(device, 0).pointer(pci::pci()).virt();

        println!("{start:x?}");

        let mut uart = Uart16550::new_with_stride(start, 1);
        uart.init(0);
        // uart.write_str("PCI Uart Hello\n"); 
    }
}

/// read: RBR, write: THR, when DLAB=1: DLL
const RBR_THR_DLL: usize = 0x0;
/// Interrupt Enable, when DLAB=1: DLM
const IER_DLM: usize = 0x1;
/// read: IIR, write: FCR
const IIR_FCR: usize = 0x2;
/// Line Control
const LCR: usize = 0x3;
/// Modem Control
const MCR: usize = 0x4;
/// Line Status
const LSR: usize = 0x5;
/// Modem Status
const MSR: usize = 0x6;

// LCR bits
const LCR_DLAB: u8 = 1 << 7;

// LSR bits
const LSR_DATA_READY: u8 = 1 << 0;
const LSR_THR_EMPTY: u8 = 1 << 5;

pub struct Uart16550 {
    base: *mut (),
    /// Register stride in bytes. Usually 1.
    stride: usize,
}

impl Uart16550 {
    /// # Safety
    /// Caller must ensure `base` points to a valid 16550 register block
    /// mapped as device memory (uncached) and that only one mutable owner exists.
    pub const unsafe fn new(base: *mut ()) -> Self {
        Self { base, stride: 1 }
    }

    /// Same, but override register stride (1 for typical 16550, 4 on some platforms).
    ///
    /// # Safety
    /// Same requirements as `new`.
    pub const unsafe fn new_with_stride(base: *mut (), stride: usize) -> Self {
        Self { base, stride }
    }

    #[inline(always)]
    unsafe fn reg_ptr(&self, offset: usize) -> *mut u8 {
        unsafe { self.base.cast::<u8>().add(offset * self.stride) }
    }

    #[inline(always)]
    fn read_reg(&self, offset: usize) -> u8 {
        unsafe { self.reg_ptr(offset).read_volatile() }
    }

    #[inline(always)]
    fn write_reg(&self, offset: usize, val: u8) {
        unsafe { self.reg_ptr(offset).write_volatile(val) }
    }

    /// Optional: initialize UART for 8N1 and set baud divisor.
    ///
    /// `divisor = input_clock_hz / (16 * baud)`
    /// For example, if input clock is 1.8432 MHz and baud is 115200, divisor=1.
    pub fn init(&mut self, divisor: u16) {
        // Disable interrupts
        self.write_reg(IER_DLM, 0x00);

        // Enable DLAB to set divisor
        let lcr = 0x03; // 8 data bits, no parity, 1 stop (8N1), DLAB=0
        self.write_reg(LCR, lcr | LCR_DLAB);

        self.write_reg(RBR_THR_DLL, (divisor & 0x00FF) as u8); // DLL
        self.write_reg(IER_DLM, ((divisor >> 8) & 0x00FF) as u8); // DLM

        // Clear DLAB, keep 8N1
        self.write_reg(LCR, lcr);

        // Enable FIFO, clear RX/TX FIFOs, set FIFO trigger level to 14 bytes (optional)
        self.write_reg(IIR_FCR, 0xC7);

        // Modem control: RTS/DSR set, OUT2 often required to enable interrupts on PC,
        // harmless for polling.
        self.write_reg(MCR, 0x0B);

        // Read LSR/MSR to clear status (optional)
        let _ = self.read_reg(LSR);
        let _ = self.read_reg(MSR);
    }

    #[inline]
    pub fn can_tx(&self) -> bool {
        (self.read_reg(LSR) & LSR_THR_EMPTY) != 0
    }

    #[inline]
    pub fn can_rx(&self) -> bool {
        (self.read_reg(LSR) & LSR_DATA_READY) != 0
    }

    pub fn putc(&mut self, c: u8) {
        while !self.can_tx() {
            core::hint::spin_loop();
        }
        self.write_reg(RBR_THR_DLL, c);
    }

    pub fn try_getc(&mut self) -> Option<u8> {
        if self.can_rx() {
            Some(self.read_reg(RBR_THR_DLL))
        } else {
            None
        }
    }

    pub fn write_bytes(&mut self, s: &[u8]) {
        for &b in s {
            self.putc(b);
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for &b in s.as_bytes() {
            if b == b'\n' {
                self.putc(b'\r');
            }
            self.putc(b);
        }
    }

    pub fn hex(&mut self, value: usize) -> &mut Self{
        for i in (0..core::mem::size_of::<usize>() * 2).rev() {
            let nibble = ((value >> (i * 4)) & 0xF) as u8;
            let c = match nibble {
                0..=9 => b'0' + nibble,
                10..=15 => b'a' + (nibble - 10),
                _ => unreachable!(),
            };
            crate::uart::uart().write_bytes(&[c]);
        }
        self
    }

    pub fn str(&mut self, s: &str) -> &mut Self{
        self.write_str(s);
        self
    }
}

impl core::fmt::Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}
