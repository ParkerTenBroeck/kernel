use crate::arch;



#[derive(Debug)]
pub struct Task{
    pub ctx: Context,
}

#[derive(Debug)]
pub struct Context{
    pub arch: arch::Context,
    pub kstack: *mut u8,
    pub mmap: (),
}