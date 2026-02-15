use core::{
    arch::global_asm,
    mem::MaybeUninit,
    ptr::{NonNull, addr_of_mut},
};

use riscv::register::{satp::Mode, scause, stvec::Stvec};

use crate::{arch::page, print, println};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Task {
    pub next: NonNull<Task>,
    pub stack: *mut u8,
    pub frame: TrapFrame,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TrapCtx {
    pub scratch: usize,
    pub task: NonNull<Task>,
}

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
        
       
        csrrw x31, sscratch, x31  // swap x31 and sscratch, x31 -> TrapCtx
        sd sp, 0(x31)            // TrapCtx->scratch = sp

        csrr sp, sstatus
        andi sp, sp, (1 << 8)   # isolate SPP bit (bit 8)

        bnez sp, .Lsupervisor
            // we are comming from user mode (untrusted stack)
            ld sp, 8(x31)            // sp = TrapCtx->task
            ld sp, 8(x31)            // sp = Task->stack
        .Lsupervisor:

        addi sp, sp, -{frame_size}

        // general registers
        sd x1, 1 * 8( sp )
        // sp
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
        sd x27, 27 * 8( sp ) // at this point we've saved all "save" registers

        ld x1, 0(x31) // load stack ptr from ctx scratch
        sd x1, 2 * 8( sp ) // save previous stack ptr

        move s0, x31             // save TrapCtx
        csrrw x31, sscratch, x31 // restore sscratch

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
        
        ld x2, 2 * 8( sp ) // restore previous stack
        
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
    println!("{frame:x?}");

    if scause.is_exception() {
        frame.pc += 4;

        let desc = match scause.code() {
            0 => "Instruction address misaligned",
            1 => "Instruction access fault",
            2 => "Illegal instruction",
            3 => "Breakpoint",
            4 => "Load address misaligned",
            5 => "Load access fault",
            6 => "Store address mimsaligned",
            7 => "Store access fault",
            8 => "Env call from U-mode",
            9 => "Env call from S-mode",
            11 => {
                // csr::
                println!(
                    "\nEnv call from M-mode hardid: \"{}\"... returning",
                    riscv::register::marchid::read().bits()
                );
                // println!("{:#?}", frame);
                // timer::mdelay(6000);
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

                unsafe fn print_thing(table: &page::PageTable, level: usize) {
                    for entry in &table.entries {
                        if entry.valid() {
                            for _ in 0..level {
                                print!(" ");
                            }
                            println!("{:?}:{:x?}", entry as *const page::PageTableEntry, entry);
                            if entry.perms() == 0 {
                                unsafe {
                                    print_thing(
                                        &*((entry.ppn() << 12) as *const page::PageTable),
                                        level + 1,
                                    )
                                }
                            }
                        }
                    }
                }
                if stap.mode() != Mode::Bare {
                    unsafe {
                        print_thing(&*((stap.ppn() << 12) as *const page::PageTable), 0);
                    }
                }

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
            0x7 => {}
            0xb => {
                // let pending = unsafe { plic::mclaim_int() };
                // use core::sync::atomic::Ordering;
                // if pending != 0 {
                //     if let Some(ptr) = PLIC_HANDLERS
                //         .get(pending as usize)
                //         .and_then(|v: &AtomicPtr<()>| NonNull::new(v.load(Ordering::Acquire)))
                //     {
                //         let func: fn() = unsafe { core::mem::transmute(ptr) };
                //         func()
                //     } else {
                //         panic!("\n\n\nplic: 0x{pending:016x}, mtval: 0x{mtval:016x}\nUnknown plic interrupt value. Cannot continue resetting\n\n");
                //     }
                //     unsafe {
                //         plic::mint_complete(pending);
                //     }
                // } else {
                //     panic!("\n\n\nscause: 0x{scause:016x}, mepc: 0x{mepc:016x}, mtval: 0x{mtval:016x}\nPlic Interrupt but no pending interrupt found? Cannot continue resetting\n\n");
                // }
            }
            _ => {
                panic!(
                    "\n\n\nscause: 0x{scause:016x?}, mepc: 0x{sepc:016x}, mtval: 0x{stval:016x}\nCannot continue resetting\n\n"
                );
            }
        }
    }
}

static mut CORE_TRAP_CTX: MaybeUninit<TrapCtx> = MaybeUninit::zeroed();
static mut INIT_TASK: MaybeUninit<Task> = MaybeUninit::zeroed();

type InitTask = unsafe extern "C" fn(hart_id: usize, dtb_ptr: *const u8) -> !;

/// # Safety
///
/// .
pub unsafe fn init(init: InitTask, hart_id: usize, dtb_ptr: *const u8) -> ! {
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

    #[allow(static_mut_refs)]
    unsafe {
        let task = NonNull::new_unchecked(INIT_TASK.as_mut_ptr());
        *addr_of_mut!((*task.as_ptr()).next) = task;
        *addr_of_mut!((*CORE_TRAP_CTX.as_mut_ptr()).task) = task;
        riscv::register::sscratch::write(CORE_TRAP_CTX.as_ptr() as usize);
    }

    let sp = crate::mem::KernelLayout::new().stack.end;

    let mut sstatus = riscv::register::sstatus::read();
    sstatus.set_spie(true);
    sstatus.set_spp(riscv::register::sstatus::SPP::Supervisor);

    let mut frame = TrapFrame {
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
            "move sp, {0}
            tail strap_return",
            in(reg) &mut frame,
            options(noreturn, nostack)
        )
    }
}
