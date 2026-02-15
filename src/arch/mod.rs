use core::arch::asm;

pub mod entry;
pub mod page;
pub mod strap;
pub mod reloc;

pub fn halt() -> ! {
    loop {
        riscv::asm::wfi()
    }
}


pub fn link_addr() -> usize{
    let out;
    unsafe{
        asm!("lga {}, KERNEL_VMA", out(reg) out);
    }
    out
}