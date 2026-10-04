use crate::{arch, sync::mutex::CriticalSpinLock};



// #[derive(Debug)]
pub struct Task{
    pub ctx: Ctx,
    pub next: *mut Task,
}

pub type Ctx = crate::alloc::sync::Arc<CriticalSpinLock<Context>>;

#[derive(Debug)]
pub struct Context{
    /// The architecture specific context
    pub arch: arch::Context,
    /// Kernel stack, if located on the heap.
    pub kstack: *mut u8,
    pub mmap: (),

    /// Time this context was switched to
    pub switch_time: u128,
    /// Amount of CPU time used
    pub cpu_time: u128,    
    /// Context should wake up at specified time
    pub wake: Option<u128>,
}