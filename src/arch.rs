

pub fn wfi() {
    unsafe {
        core::arch::asm!("wfi");
    }
}

pub fn halt() -> !{
    loop{
        wfi()
    }
}