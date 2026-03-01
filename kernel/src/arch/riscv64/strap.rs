use core::arch::{asm, global_asm};

use riscv::register::{scause, stvec::Stvec};

use crate::{arch::Frame, println};


#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PerCpu{
    pub scratch: usize,
    pub kernel_sp: *mut u8,
    pub hart_id: usize,
}


global_asm!(
        r#"
    .globl  strap_vector

    .section .text.strap_vector,"ax",@progbits
    .globl strap_vector

    .balign 4
    strap_vector:
        
       
        csrrw tp, sscratch, tp  // swap tp and sscratch
        
        bnez tp, .Lsave_user_context
        .Lsave_kernel_context:

                csrr tp, sscratch       // restore tp
                sd sp, 0(tp)            // save kernel stack pointer to scratch

                j .Lsave_context

        .Lsave_user_context:

            sd sp, 0(tp)    // save "user" sp to task scratch
            ld sp, 8(tp)    // load kernel sp

        .Lsave_context:

        addi sp, sp, -{frame_size}

        // general registers
        sd x1, 1 * 8( sp )
        // sp
        sd x3, 3 * 8( sp )
        // tp
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

        ld s0, 0(tp)        // load sp from scratch
        sd s0, 2 * 8( sp )  // save it to the stack
        
        csrr s1, sscratch   // read tp
        sd s1, 4 * 8( sp )  // save it to the stack


        csrw sscratch, x0               // set to 0 so if a recursive exception occurs we know we're comming from kernel space

        csrr t0, sstatus
        sd t0, 32 * 8( sp )

        addi a0, sp, 0
        csrr a1, scause
        csrr a2, sepc
        csrr a3, stval

        // save PC
        sd a2, 0( sp )

        jal {handler}





.globl  strap_return

.section .text.strap_return,"ax",@progbits
.globl strap_return

        strap_return:


        ld t0, 32 * 8(sp)
        csrw sstatus, t0


        andi t0, t0, 0x100 // are we transitioning to supervisor?

        bnez t0, 1f

            // we're going into user mode. save some info
            addi t0, sp, {frame_size}
            sd t0, 8(tp)                // save kernel stack pointer to task struct
            csrw sscratch, tp            // save task struct pointer to scratch

        1:


        ld t0, 0(sp)
        csrw sepc, t0

        
        ld x1, 1 * 8( sp )
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
        
        ld x2, 2 * 8( sp )

        sret
    "#,
    handler = sym strap_handler,
    frame_size = const core::mem::size_of::<Frame>(),
);

pub extern "C" fn strap_handler(
    frame: &mut Frame,
    scause: scause::Scause,
    sepc: usize,
    stval: usize,
) {
    if scause.is_exception() {
        println!("{frame:x?}");
        let instr_enc = unsafe { (sepc as *const u16).read_volatile() };
        if instr_enc & 0b11 != 0b11 {
            frame.pc += 2;
        } else if instr_enc & 0b11100 != 0b11100 {
            frame.pc += 4;
        } else if instr_enc & 0b111111 != 0b011111 {
            frame.pc += 6;
        } else if instr_enc & 0b1111111 != 0b0111111 {
            frame.pc += 8;
        } else if instr_enc & 0b1111111 == 0b1111111 {
            frame.pc += 10 + 16 * ((instr_enc >> 12) as usize & 0b111);
        }

        let desc = match scause.code() {
            0 => "Instruction address misaligned",
            1 => "Instruction access fault",
            2 => "Illegal instruction",
            3 => {
                println!("Breakpoint");
                return;
            }
            4 => "Load address misaligned",
            5 => "Load access fault",
            6 => "Store address mimsaligned",
            7 => "Store access fault",
            8 => "Env call from U-mode",
            9 => "Env call from S-mode",
            11 => {
                println!(
                    "\nEnv call from M-mode hardid: \"{}\"... returning",
                    riscv::register::marchid::read().bits()
                );
                return;
            }
            12 | 13 | 15 => {
                let desc = match scause.code() {
                    12 => "Instruction page fault",
                    13 => "Page fault on load",
                    15 => "Page fault on store",
                    _ => "",
                };

                let stap = riscv::register::satp::read();
                println!("{:?}, {:?}, 0x{:x?}", stap.mode(), stap.asid(), stap.ppn());

                panic!(
                    "\n\n\n{desc}:\nscause: {scause:016x?}, mepc: 0x{sepc:016x}, mtval: 0x{stval:016x}, \nCannot continue resetting\n\n"
                );
            }
            _ => "Unknown exception",
        };
        panic!(
            "\n\n\n{desc}:\nscause: {scause:016x?}, mepc: 0x{sepc:016x}, mtval: 0x{stval:016x}, \nCannot continue resetting\n\n"
        );
    } else {
        match scause.code() {
            0x5 => {
                println!("Timer Interrupt");
            }
            0x9 => {
                panic!("External S-Mode interrupt");
            }
            0xb => {
                panic!("External M-Mode interrupt");
            }
            _ => {
                panic!(
                    "\n\n\nscause: 0x{scause:016x?}, mepc: 0x{sepc:016x}, mtval: 0x{stval:016x}\nCannot continue resetting\n\n"
                );
            }
        }
    }
}

/// # Safety
///
/// .
pub unsafe fn init(hart_id: usize){
    unsafe extern "C" {
        #[link_name = "strap_vector"]
        pub fn strap_vector();
    }
    unsafe {
        riscv::register::sscratch::write(0);
        riscv::register::stvec::write(Stvec::new(
            strap_vector as *const () as usize,
            riscv::register::stvec::TrapMode::Direct,
        ));
        use crate::alloc::boxed::Box;
        let ptr = Box::leak(Box::new(PerCpu{
            scratch: 0,
            kernel_sp: core::ptr::null_mut(),
            hart_id,
        }));
        asm!("move tp, {0}", in(reg) ptr);
        riscv::asm::ebreak();
        riscv::asm::ebreak();
    }
}

type InitTask = unsafe extern "C" fn(hart_id: usize, dtb_ptr: *const u8) -> !;

/// # Safety
///
/// .
pub unsafe fn begin_init_task(init: InitTask, hart_id: usize, dtb_ptr: *const u8) -> ! {

    let sp = crate::mem::KernelLayout::new().stack.end;

    let mut sstatus = riscv::register::sstatus::read();
    sstatus.set_spie(true);
    sstatus.set_spp(riscv::register::sstatus::SPP::Supervisor);

    let mut frame = Frame {
        pc: init as usize,
        regs: [0; 31],
        sstatus,
    };
    frame.regs[1] = sp;
    frame.regs[9] = hart_id;
    frame.regs[10] = dtb_ptr as usize;

    println!("Beginning Init Task");
    unsafe {
        core::arch::asm!(

            "
            move sp, {0}
            tail strap_return",
            in(reg) &mut frame,
            options(noreturn, nostack)
        )
    }
}
