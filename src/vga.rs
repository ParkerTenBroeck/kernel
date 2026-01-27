
use crate::{dtb::*, pci::pci, println};

pub struct FrameBuffer(*mut u8);



pub fn init(){

    println!("Setting up VGA device");

    let Some((device, id)) = pci().find_device_vendor(0x1234) else {
        panic!("storage device not found")
    };

    // enable mmio and busmaster
    unsafe{
        let command = pci().pointer(device, 0x04);

        let v = command.read_unaligned();
        let cmd = (v & 0xFFFF) as u16;
        let new_cmd = cmd | (1 << 1) | (1 << 2);
        let new_v = (v & 0xFFFF_0000) | (new_cmd as u32);
        command.write_volatile(new_v);
    }

    for bar in 0..=5{
        println!("bar{bar}: {:?}", unsafe {pci().read_bar(device, bar)})
    }


    let bar2 = unsafe {pci().read_bar(device, 2)};
    let cfg_base = bar2.expect_mmio() as *mut u16;

    println!("vga cfg base {cfg_base:?}");

    // Commonly used Bochs Dispi indices (widely documented)
    const BGA_XRES: u16 = 0x01;
    const BGA_YRES: u16 = 0x02;
    const BGA_BPP:  u16 = 0x03;
    const BGA_ENABLE: u16 = 0x04;

    // Enable flags (typical)
    const BGA_ENABLED: u16 = 0x0001;
    const BGA_LFB:     u16 = 0x0040; // linear framebuffer

    unsafe{
        cfg_base.add(0x400 + BGA_ENABLE as usize).write_volatile(0);
        cfg_base.add(0x400 + BGA_XRES as usize).write_volatile(640);
        cfg_base.add(0x400 + BGA_YRES as usize).write_volatile(480);
        cfg_base.add(0x400 + BGA_BPP as usize).write_volatile(16);
        cfg_base.add(0x400 + BGA_ENABLE as usize).write_volatile(BGA_ENABLED | BGA_LFB);
    }


    //bar 0
    let bar0 = unsafe {pci().read_bar(device, 0)};
    let fb_base = bar0.expect_mmio() as *mut u8;

    println!("vga framebuffer base {fb_base:?}");

}