pub mod entry;
pub mod mtrap;
pub mod page;
pub mod strap;

pub fn halt() -> ! {
    loop {
        riscv::asm::wfi()
    }
}
