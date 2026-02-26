use crate::arch;




pub struct Task{

}

pub struct Context{
    pub arch: arch::Context,
    pub kstack: *mut u8,
}