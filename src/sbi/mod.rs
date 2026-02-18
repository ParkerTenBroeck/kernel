#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SbiRet {
    pub error: isize,
    pub value: isize,
}

#[inline(always)]
#[allow(unsafe_op_in_unsafe_fn)]
#[allow(clippy::too_many_arguments)]
unsafe fn sbi_ecall(
    ext: usize,
    fid: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> SbiRet {
    let error: isize;
    let value: isize;
    core::arch::asm!(
        "ecall",
        inlateout("a0") arg0 as isize => error,
        inlateout("a1") arg1 as isize => value,
        in("a2") arg2,
        in("a3") arg3,
        in("a4") arg4,
        in("a5") arg5,
        in("a6") fid,
        in("a7") ext,
        options(nostack, preserves_flags),
    );
    SbiRet { error, value }
}

const SBI_EXT_TIME: usize = 0x54494D45; // "TIME"
const SBI_FID_SET_TIMER: usize = 0;

pub fn sbi_set_timer(deadline: u64) {
    unsafe {
        let ret = sbi_ecall(
            SBI_EXT_TIME,
            SBI_FID_SET_TIMER,
            deadline as usize,
            0,
            0,
            0,
            0,
            0,
        );
        debug_assert!(ret.error == 0);
    }
}
