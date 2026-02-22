use core::arch::{asm, global_asm};

use riscv::register::{scause, stvec::Stvec};

use crate::{println};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ThreadInfo {
    pub pc: usize,
    pub regs: [usize; 31],
    pub sstatus: riscv::register::sstatus::Sstatus,
}

impl Default for ThreadInfo{
    fn default() -> Self {
        Self { 
            pc: Default::default(), 
            regs: Default::default(), 
            sstatus: riscv::register::sstatus::Sstatus::from_bits(0)
        }
    }
}

global_asm!(
        r#"
    .globl  strap_vector

    .section .text.strap_vector,"ax",@progbits
    .globl strap_vector

    .balign 4
    strap_vector:
        
       
        csrrw tp, sscratch, tp  // swap tp and sscratch
        beqz tp, .Lkernel

        j .Lend
        .Lkernel:
                csrrw tp, sscratch, tp  // restore tp and sscratch

                addi sp, sp, -{frame_size}
                
                sd x2, 2 * 8( sp )
        .Lend:


        // general registers
        sd x1, 1 * 8( sp )
        
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
        csrr a4, sscratch

        // save PC
        sd a2, 0( sp )

        jal {handler}

        strap_return:

        ld t0, 0(sp)
        csrw sepc, t0

        ld t0, 32 * 8(sp)
        csrw sstatus, t0

        
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
        
        addi sp, sp, {frame_size}

        sret
    "#,
    handler = sym strap_handler,
    frame_size = const core::mem::size_of::<ThreadInfo>(),
);

pub extern "C" fn strap_handler(
    frame: &mut ThreadInfo,
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

#[derive(Default)]
struct Task{
    pub kernel_sp: *mut u8,
    pub user_sp: *mut u8,
    pub thread: ThreadInfo,
}

/// # Safety
///
/// .
pub unsafe fn init(){
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
        let ptr = Box::leak(Box::new(Task::default()));
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

    let mut frame = ThreadInfo {
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
            ebreak
            move sp, {0}
            tail strap_return",
            in(reg) &mut frame,
            options(noreturn, nostack)
        )
    }
}
