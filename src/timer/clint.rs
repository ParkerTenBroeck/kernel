use core::ptr::{read_volatile, write_volatile};

use crate::{
    dtb::{ByteStream, DtbNodes, DtbProperties},
    interrupt::plic::{Plic, PlicDev},
    mem::Pointer,
    println,
};

#[derive(Copy, Clone)]
pub struct Clint {
    base: usize,
}

impl Clint {
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    #[inline(always)]
    const fn msip_addr(&self, hart: usize) -> *mut u32 {
        (self.base + 0x0000 + hart * 4) as *mut u32
    }

    #[inline(always)]
    const fn mtimecmp_addr(&self, hart: usize) -> *mut u64 {
        (self.base + 0x4000 + hart * 8) as *mut u64
    }

    #[inline(always)]
    const fn mtime_addr(&self) -> *const u64 {
        (self.base + 0xBFF8) as *const u64
    }

    // -------------------------
    // IPIs (MSIP)
    // -------------------------

    /// Send a machine software interrupt (IPI) to `hart`.
    ///
    /// If you delegate MSIP to S-mode and map CLINT into S-mode, this can be used
    /// from supervisor as well (it’s still the same MSIP bit).
    #[inline(always)]
    pub unsafe fn send_ipi(&self, hart: usize) {
        unsafe { write_volatile(self.msip_addr(hart), 1) }
    }

    /// Clear the software interrupt pending bit for `hart`.
    #[inline(always)]
    pub unsafe fn clear_ipi(&self, hart: usize) {
        unsafe { write_volatile(self.msip_addr(hart), 0) }
    }

    /// Read MSIP state for `hart` (0 or 1).
    #[inline(always)]
    pub unsafe fn ipi_pending(&self, hart: usize) -> bool {
        unsafe { read_volatile(self.msip_addr(hart)) & 1 != 0 }
    }

    // -------------------------
    // Timer (MTIME / MTIMECMP)
    // -------------------------

    /// Read the 64-bit MTIME counter.
    ///
    /// On RV64 this is a single 64-bit MMIO load.
    /// On RV32, MTIME is 64-bit but must be read atomically with hi/lo technique.
    #[inline(always)]
    pub unsafe fn read_mtime(&self) -> u64 {
        #[cfg(target_pointer_width = "64")]
        unsafe {
            read_volatile(self.mtime_addr())
        }

        #[cfg(target_pointer_width = "32")]
        unsafe {
            // MTIME is at 0xBFF8; on 32-bit we must read hi/lo/hi until stable.
            let lo_ptr = (self.base + 0xBFF8) as *const u32;
            let hi_ptr = (self.base + 0xBFFC) as *const u32;

            loop {
                let hi1 = read_volatile(hi_ptr);
                let lo = read_volatile(lo_ptr);
                let hi2 = read_volatile(hi_ptr);
                if hi1 == hi2 {
                    return ((hi1 as u64) << 32) | (lo as u64);
                }
            }
        }
    }

    /// Program MTIMECMP for `hart` to fire when MTIME reaches `deadline`.
    ///
    /// If M-mode delegates MTIP to S-mode (mideleg bit 7) *and* you allow S-mode
    /// to access CLINT, supervisor code can use this directly.
    #[inline(always)]
    pub unsafe fn set_timer_deadline(&self, hart: usize, deadline: u64) {
        unsafe {
            write_volatile(self.mtimecmp_addr(hart), u64::MAX);
            write_volatile(self.mtimecmp_addr(hart), deadline);
        }
    }

    /// Disable timer interrupts for `hart` by setting MTIMECMP to max.
    #[inline(always)]
    pub unsafe fn disable_timer(&self, hart: usize) {
        unsafe { write_volatile(self.mtimecmp_addr(hart), u64::MAX) }
    }

    /// Convenience: set a timer interrupt `ticks_from_now` in the future.
    #[inline(always)]
    pub unsafe fn set_timer_relative(&self, hart: usize, ticks_from_now: u64) {
        unsafe {
            let now = self.read_mtime();
            self.set_timer_deadline(hart, now.wrapping_add(ticks_from_now));
        }
    }
}

#[allow(static_mut_refs)]
pub fn init(dtb: &crate::dtb::Dtb) {
    for plic in dtb.nodes().compatible(b"riscv,plic0") {
        let [start, _size] = plic.properties().expect_value(b"reg", |stream| {
            stream.usize_cells_arr(dtb.root().addr_size_cells())
        });
        let max_int = plic
            .properties()
            .expect_value(b"riscv,ndev", ByteStream::u32);
        unsafe {
            let mut plic = PlicDev::new(Pointer::from_phys(start as *mut Plic).virt(), max_int);

            plic.clear();
        }
    }

    let timebase_freq = dtb
        .nodes()
        .nammed(b"cpus")
        .next()
        .expect("expected cpu")
        .properties()
        .expect_value(b"timebase-frequency", ByteStream::u32);

    for clint in dtb.nodes().compatible(b"riscv,clint0") {
        let [start, _size] = clint.properties().expect_value(b"reg", |stream| {
            stream.usize_cells_arr(dtb.root().addr_size_cells())
        });

        unsafe {
            // println!("{:?}", riscv::register::medeleg::read());

            let ptr = Pointer::from_phys(start as *mut ()).virt() as usize;
            let clint = Clint::new(ptr);

            clint.set_timer_relative(0, timebase_freq as u64);

            // sbi_set_timer(riscv::register::time::read64() + 10000000);
            riscv::register::sie::set_stimer();
            // riscv::register::sie::set_ssoft();
            riscv::register::sie::set_sext();
            riscv::register::sstatus::set_sie();

            // clint.send_ipi(0);

            println!("meow");
        }
    }
}
