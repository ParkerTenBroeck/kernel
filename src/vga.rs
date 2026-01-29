
use crate::{hexdump::{hexdump_u8, hexdump_u16, hexdump_u32}, pci::{CommandRegister, pci}, println};


#[repr(C, align(4))]
pub struct Color{
    b: u8,
    g: u8,
    r: u8,
    _a: u8
}
pub struct FrameBuffer(*mut Color);



pub fn init(){
    println!("Setting up VGA device");

    let (cfg_base, fb) = init_pci();

    let xres = 1280;
    let yres = 720;
    init_boochs(cfg_base, xres, yres);

    assert!(0x1000000 >= xres as usize * yres as usize);

    for y in 0..yres {
        for x in 0..xres {
            unsafe { 
                fb.0.add(y as usize * xres as usize + x as usize).write_volatile(Color { 
                    r: x as u8, 
                    g: y as u8, 
                    b: (x+y) as u8, 
                    _a: 0
                }); 
            }
        }
    }

    println!("Initialized VGA")
}

fn init_pci() -> (*mut u16, FrameBuffer){
    let Some((device, _)) = pci().find_device_vendor(0x1234, 0x1111) else {
        panic!("display device not found")
    };

    
    unsafe{
        let (_, command) = pci().read_cmd_status(device);

        
        pci().write_cmd_status(device, 
            *command.clone()
            .set(CommandRegister::BUS_MASTER, true)
            .set(CommandRegister::IO_SPACE, false)
            .set(CommandRegister::MEMORY_SPACE, false)
        );

        pci().allocate_bar(device, 0);
        pci().allocate_bar(device, 2);

        pci().write_cmd_status(device, 
            *command.clone()
            .set(CommandRegister::BUS_MASTER, true)
            .set(CommandRegister::IO_SPACE, false)
            .set(CommandRegister::MEMORY_SPACE, true)
        );        
    }


    let bar2 = unsafe {pci().read_bar(device, 2)};
    let cfg_base = bar2.address() as *mut u16;
    println!("vga cfg base {cfg_base:?}");

    let bar0 = unsafe {pci().read_bar(device, 0)};
    let fb_base = bar0.address() as *mut Color;
    println!("vga framebuffer base {fb_base:?}");

    (cfg_base, FrameBuffer(fb_base))
}

fn init_boochs(cfg_base: *mut u16, xres: u16, yres: u16){
    const BGA_ID:         u16 = 0x00;
    const BGA_XRES:       u16 = 0x01;
    const BGA_YRES:       u16 = 0x02;
    const BGA_BPP:        u16 = 0x03;
    const BGA_ENABLE:     u16 = 0x04;
    const BGA_BANK:       u16 = 0x05;
    const BGA_VIRT_WIDTH: u16 = 0x06;
    const BGA_VIRT_HEIGHT:     u16 = 0x07;
    const BGA_X_OFFSET:   u16 = 0x08;
    const BGA_Y_OFFSET:   u16 = 0x09;

    const BGA_ENABLED: u16 = 0x0001;
    const BGA_LFB:     u16 = 0x0040;

    let vga_cfg_out = |index: u16, data: u16|{
        unsafe{
            cfg_base.byte_add(0x500).add(index as usize).write_volatile(data);
        }
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
}