use core::arch::global_asm;

use riscv::register::{scause, stvec::Stvec};

use crate::{dtb, println};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TrapFrame {
    pub pc: usize,
    pub regs: [usize; 31],
    pub sstatus: riscv::register::sstatus::Sstatus,
}

global_asm!(
        r#"
    .globl  strap_vector

    .section .text.strap_vector,"ax",@progbits
    .globl strap_vector

    .balign 4
    strap_vector:
        
        addi sp, sp, -{frame_size}

        sd x1, 1 * 8( sp )
        sd x1, 2 * 8( sp ) // csrr x1, sscratch
        sd x3, 3 * 8( sp )
        sd x4, 4 * 8( sp )
        sd x5, 5 * 8( sp )
        sd x6, 6 * 8( sp )
        sd x7, 7 * 8( sp )
        sd x8, 8 * 8( sp )
        sd x9, 9 * 8( sp )
        sd x10, 10 * 8( sp )
        sd x11, 11 * 8( sp )
        sd x12, 12 * 8( sp )
        sd x13, 13 * 8( sp )
        sd x14, 14 * 8( sp )
        sd x15, 15 * 8( sp )
        sd x16, 16 * 8( sp )
        sd x17, 17 * 8( sp )
        sd x18, 18 * 8( sp )
        sd x19, 19 * 8( sp )
        sd x20, 20 * 8( sp )
        sd x21, 21 * 8( sp )
        sd x22, 22 * 8( sp )
        sd x23, 23 * 8( sp )
        sd x24, 24 * 8( sp )
        sd x25, 25 * 8( sp )
        sd x26, 26 * 8( sp )
        sd x27, 27 * 8( sp )
        sd x28, 28 * 8( sp )
        sd x29, 29 * 8( sp )
        sd x30, 30 * 8( sp )
        sd x31, 31 * 8( sp )

        csrr t0, sstatus
        sd t0, 32 * 8( sp )

        addi a0, sp, 0
        csrr a1, scause
        csrr a2, sepc
        csrr a3, stval

        # test if asynchronous
        srli t0, a1, 64 - 1		/* MSB of scause is 1 if handing an asynchronous interrupt - shift to LSB to clear other bits. */
        beq t0, x0, s.handle_synchronous		/* Branch past interrupt handing if not asynchronous. */
        	

    s.handle_asynchronous:
        sd a2, 0( sp )
        jal {handler}
        j s.return

    s.handle_synchronous:
        addi t0, a2, 4
        sd t0, 0( sp )
        jal {handler}


    s.return:

        ld t0, 0(sp)
        csrw sepc, t0

        ld t0, 32 * 8(sp)
        csrw sstatus, t0

        
        ld x1, 1 * 8( sp )
        ld x2, 2 * 8( sp )
        ld x3, 3 * 8( sp )
        ld x4, 4 * 8( sp )
        ld x5, 5 * 8( sp )
        ld x6, 6 * 8( sp )
        ld x7, 7 * 8( sp )
        ld x8, 8 * 8( sp )
        ld x9, 9 * 8( sp )
        ld x10, 10 * 8( sp )
        ld x11, 11 * 8( sp )
        ld x12, 12 * 8( sp )
        ld x13, 13 * 8( sp )
        ld x14, 14 * 8( sp )
        ld x15, 15 * 8( sp )
        ld x16, 16 * 8( sp )
        ld x17, 17 * 8( sp )
        ld x18, 18 * 8( sp )
        ld x19, 19 * 8( sp )
        ld x20, 20 * 8( sp )
        ld x21, 21 * 8( sp )
        ld x22, 22 * 8( sp )
        ld x23, 23 * 8( sp )
        ld x24, 24 * 8( sp )
        ld x25, 25 * 8( sp )
        ld x26, 26 * 8( sp )
        ld x27, 27 * 8( sp )
        ld x28, 28 * 8( sp )
        ld x29, 29 * 8( sp )
        ld x30, 30 * 8( sp )
        ld x31, 31 * 8( sp )

        addi sp, sp, {frame_size}
        
        sret
    "#,
    handler = sym strap_handler,
    frame_size = const core::mem::size_of::<TrapFrame>(),
);

pub extern "C" fn strap_handler(
    frame: &mut TrapFrame,
    scause: scause::Scause,
    sepc: usize,
    stval: usize,
) {
    println!("{scause:x?} {sepc:x?} {stval:x?} {frame:#?}")
}

pub fn init(dtb: &dtb::Dtb) {
    let _ = dtb;
    println!("Enabling S-Mode interrupts");

    unsafe extern "C" {
        #[link_name = "strap_vector"]
        pub fn strap_vector();
    }
    unsafe {
        riscv::register::stvec::write(Stvec::new(
            strap_vector as *const () as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
    }

    unsafe {
        riscv::register::sie::set_sext();
        riscv::register::sie::set_stimer();
        riscv::register::sie::set_ssoft();
        riscv::register::sstatus::set_sie();
    }
    println!("Interrupts enabled S-Mode ");
}
