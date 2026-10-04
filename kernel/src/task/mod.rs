use crate::arch;



#[derive(Debug)]
pub struct Task{
    pub ctx: Context,
    pub next: *mut Task,
}

#[derive(Debug)]
pub struct Context {
    pub arch: arch::Context,
    pub mmap: (),
}