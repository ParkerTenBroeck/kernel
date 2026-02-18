use core::arch::asm;

use riscv::register::scause;

pub mod entry;
pub mod mtrap;
pub mod page;
pub mod reloc;
pub mod strap;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryError {
    pub sepc: usize,
    pub stval: usize,
    pub scause: scause::Scause,
}

/// # Safety
/// .
pub unsafe fn riscv_try<T>(func: impl FnOnce() -> T) -> Result<T, TryError> {
    let previous = riscv::register::stvec::read();
    unsafe {
        asm!("
            
            lla t0, {err}
            csrw stvec, t0

            j {func}
            ",
            func = label {
                let res =  Ok(func());
                unsafe{riscv::register::stvec::write(previous);}
                return res;
            },
            err = label {
                unsafe{
                    riscv::register::stvec::write(previous);
                    let err = TryError {
                        sepc: riscv::register::sepc::read(),
                        stval: riscv::register::stval::read(),
                        scause: riscv::register::scause::read()
                    };
                    asm!("
                        lla t0, 0f
                        csrw sepc, t0
                        sret
                        0:
                    ");
                    return Err(err)
                }
            },
        )
    }
    unreachable!()
}
