use core::arch::asm;

pub mod entry;
pub mod page;
pub mod reloc;
pub mod strap;
pub mod trace;

pub fn halt() -> ! {
    loop {
        riscv::asm::wfi()
    }
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
pub struct Context {
    pub frame: Frame
}



#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PerCpu {
    pub scratch: usize,
    pub kernel_sp: *mut u8,
    pub hart_id: usize,

    
}

pub fn local_ctx() -> *mut PerCpu {
    let tp;
    unsafe {
        // Read directly from the `tp` register into a general-purpose register
        asm!(
            "mv {}, tp", 
            out(reg) tp,
            options(nomem, nostack, preserves_flags)
        );
    }
    tp
}