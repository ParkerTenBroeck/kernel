use core::arch::asm;

pub mod entry;
pub mod mtrap;
pub mod page;
pub mod reloc;
pub mod strap;
pub mod trace;

pub fn halt() -> ! {
    loop {
        riscv::asm::wfi()
    }
}

pub fn link_addr() -> usize {
    let out;
    unsafe {
        asm!("lga {}, KERNEL_VMA", out(reg) out);
    }
    out
}



#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub pc: usize,
    pub regs: [usize; 31],
    pub sstatus: riscv::register::sstatus::Sstatus,
}

impl Default for Frame{
    fn default() -> Self {
        Self { 
            pc: Default::default(), 
            regs: Default::default(), 
            sstatus: riscv::register::sstatus::Sstatus::from_bits(0)
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Context{
    pub frame: Frame
}