use crate::dtb::{ByteStream, DtbNodes, DtbProperties};

pub struct SbiTimer {
    timebase_freq: u64,
}
impl SbiTimer {
    pub fn new(dtb: &crate::dtb::Dtb<'_>) -> Self {

    let timebase_freq = dtb
        .nodes()
        .nammed(b"cpus")
        .next()
        .expect("expected cpu")
        .properties()
        .expect_value(b"timebase-frequency", ByteStream::u32) as u64;
        Self { timebase_freq }
    }
}

impl super::TimerDev for SbiTimer {
    fn timebase_freq(&self) -> u64 {
        self.timebase_freq
    }

    fn time(&self) -> u64 {
        riscv::register::time::read64()
    }

    fn set_deadline(&self, deadline: u64) {
        crate::sbi::sbi_set_timer(deadline);
    }

    fn set_enable(&self, enabled: bool) {
        unsafe {
            if enabled {
                riscv::register::sie::set_stimer();
            } else {
                riscv::register::sie::clear_stimer();
            }
        }
    }

    fn get_enabled(&self) -> bool {
        riscv::register::sie::read().stimer()
    }
}