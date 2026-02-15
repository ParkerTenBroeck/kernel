use crate::{
    pci::{CommandRegister, pci},
    println,
};

#[repr(C, align(4))]
#[derive(Clone, Copy, Hash, Debug, Default)]
pub struct Color {
    b: u8,
    g: u8,
    r: u8,
    _a: u8,
}
impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { b, g, r, _a: 0 }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FrameBuffer {
    xres: usize,
    yres: usize,
    len: usize,
    ptr: *mut Color,
}

impl FrameBuffer {
    /// Creates a new [`FrameBuffer`].
    ///
    /// # Safety
    ///
    /// ptr must point to a valid aligned region of memory of at least size xres * yres
    /// and be valid for the lifetime of the created structure
    pub const unsafe fn new(ptr: *mut Color, xres: usize, yres: usize) -> Self {
        Self {
            xres,
            yres,
            len: xres * yres,
            ptr,
        }
    }

    pub const fn xres(&self) -> usize {
        self.xres
    }

    pub const fn yres(&self) -> usize {
        self.yres
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set(&self, x: usize, y: usize, color: Color) {
        let index = x + y * self.xres;
        if index < self.len {
            unsafe {
                self.ptr.add(index).write(color);
            }
        }
    }

    pub fn clear(&self, color: Color) {
        for y in 0..self.yres {
            for x in 0..self.xres {
                unsafe {
                    self.ptr.add(y * self.xres + x).write_volatile(color);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VGA {
    _cfg_base: *mut (),
    fb: FrameBuffer,
}

static mut VGA: VGA = VGA {
    _cfg_base: core::ptr::null_mut(),
    fb: FrameBuffer {
        ptr: core::ptr::null_mut(),
        xres: 0,
        yres: 0,
        len: 0,
    },
};

pub fn init(xres: u16, yres: u16) {
    println!("Setting up VGA device");

    let (cfg_base, fb) = unsafe {
        let (cfg_base, fb) = init_pci(xres as usize * yres as usize * 4);

        vga_minimal_init(cfg_base.cast());
        init_bochs(cfg_base.cast(), xres, yres);
        (cfg_base, fb)
    };

    let fb = unsafe { FrameBuffer::new(fb, xres as usize, yres as usize) };

    let vga = VGA {
        _cfg_base: cfg_base,
        fb,
    };
    vga.fb.clear(Color::default());

    unsafe {
        VGA = vga;
    }
    println!("Initialized {vga:#?}");
}

unsafe fn init_pci(buffer_size: usize) -> (*mut (), *mut Color) {
    let Some((device, _)) = pci().find_device_vendor(0x1234, 0x1111) else {
        panic!("display device not found")
    };

    unsafe {
        let (_, command) = pci().read_cmd_status(device);

        pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(CommandRegister::BUS_MASTER, true)
                .set(CommandRegister::IO_SPACE, false)
                .set(CommandRegister::MEMORY_SPACE, false),
        );

        let layout = pci().allocate_bar(device, 0);
        assert!(layout.size() >= buffer_size);
        pci().allocate_bar(device, 2);

        pci().write_cmd_status(
            device,
            *command
                .clone()
                .set(CommandRegister::BUS_MASTER, true)
                .set(CommandRegister::IO_SPACE, false)
                .set(CommandRegister::MEMORY_SPACE, true),
        );
    }

    let bar2 = unsafe { pci().read_bar(device, 2) };
    let cfg_base = bar2.pointer(pci()).virt();
    println!("vga cfg base {cfg_base:?}");

    let bar0 = unsafe { pci().read_bar(device, 0) };
    let fb_base = bar0.pointer(pci()).virt();
    println!("vga framebuffer base {fb_base:?}");

    (cfg_base, fb_base)
}

unsafe fn vga_minimal_init(cfg_base: *mut u8) {
    unsafe fn port_ptr(base: *mut u8, port: u16) -> *mut u8 {
        unsafe { base.add((port - 0x3C0) as usize) }
    }

    #[inline(always)]
    unsafe fn mmio_out8(vga_ports: *mut u8, port: u16, val: u8) {
        unsafe { port_ptr(vga_ports, port).write_volatile(val) }
    }

    #[inline(always)]
    unsafe fn mmio_in8(vga_ports: *mut u8, port: u16) -> u8 {
        unsafe { port_ptr(vga_ports, port).read_volatile() }
    }

    unsafe {
        let vga_ports = cfg_base.add(0x400);

        // Set MISC output: choose color emulation + enable access to 0x3D4 regs.
        // Typical value: 0x67 for 25MHz/28MHz + enable RAM + IO select.
        mmio_out8(vga_ports, 0x3C2, 0x67);

        // Sequencer: reset, then enable
        mmio_out8(vga_ports, 0x3C4, 0x00); // seq index 0 (reset)
        mmio_out8(vga_ports, 0x3C5, 0x01); // async reset
        mmio_out8(vga_ports, 0x3C5, 0x03); // sync reset released (some do 0x03)

        // Unblank display via Attribute Controller
        // Reading 0x3DA resets the flip-flop
        let _ = mmio_in8(vga_ports, 0x3DA);

        // Attribute Controller index 0x10 = Mode Control
        mmio_out8(vga_ports, 0x3C0, 0x10);
        mmio_out8(vga_ports, 0x3C0, 0x01); // graphics enable-ish baseline

        // Attribute Controller index 0x14 = Color Select (optional)
        mmio_out8(vga_ports, 0x3C0, 0x14);
        mmio_out8(vga_ports, 0x3C0, 0x00);

        // Finally "enable video" by setting bit 5 in attribute index write
        let _ = mmio_in8(vga_ports, 0x3DA);
        mmio_out8(vga_ports, 0x3C0, 0x20); // bit5=1 enables display, index=0

        // Unlock CRTC regs (common requirement)
        mmio_out8(vga_ports, 0x3D4, 0x11);
        let v = mmio_in8(vga_ports, 0x3D5);
        mmio_out8(vga_ports, 0x3D5, v & !0x80); // clear bit7 to unlock
    }

    println!("VGA port registers configured");
}

fn init_bochs(cfg_base: *mut u16, xres: u16, yres: u16) {
    const BGA_ID: u16 = 0x00;
    const BGA_XRES: u16 = 0x01;
    const BGA_YRES: u16 = 0x02;
    const BGA_BPP: u16 = 0x03;
    const BGA_ENABLE: u16 = 0x04;
    const BGA_BANK: u16 = 0x05;
    const BGA_VIRT_WIDTH: u16 = 0x06;
    const BGA_X_OFFSET: u16 = 0x08;
    const BGA_Y_OFFSET: u16 = 0x09;

    const BGA_ENABLED: u16 = 0x0001;
    const BGA_LFB: u16 = 0x0040;

    let vga_cfg_out = |index: u16, data: u16| unsafe {
        cfg_base
            .byte_add(0x500)
            .add(index as usize)
            .write_volatile(data);
    };
    let vga_cfg_in = |index: u16| -> u16 {
        unsafe { cfg_base.byte_add(0x500).add(index as usize).read_volatile() }
    };

    vga_cfg_out(BGA_ENABLE, 0);

    vga_cfg_out(BGA_BANK, 0);
    vga_cfg_out(BGA_X_OFFSET, 0);
    vga_cfg_out(BGA_Y_OFFSET, 0);

    vga_cfg_out(BGA_VIRT_WIDTH, xres);

    vga_cfg_out(BGA_XRES, xres);
    vga_cfg_out(BGA_YRES, yres);

    vga_cfg_out(BGA_BPP, 0x20);

    vga_cfg_out(BGA_ENABLE, BGA_ENABLED | BGA_LFB);

    println!("BGA_ID    = {:#x}", vga_cfg_in(BGA_ID));
    println!("XRES      = {}", vga_cfg_in(BGA_XRES));
    println!("YRES      = {}", vga_cfg_in(BGA_YRES));
    println!("BPP       = {}", vga_cfg_in(BGA_BPP));
    println!("STRIDE    = {}", vga_cfg_in(BGA_VIRT_WIDTH));
    println!("EN        = {:#x}", vga_cfg_in(BGA_ENABLE));

    let stride = vga_cfg_in(BGA_VIRT_WIDTH);
    assert_eq!(stride, xres);

    println!("VGA Boch Initialized");
}

pub const fn framebuffer() -> FrameBuffer {
    unsafe { VGA.fb }
}
