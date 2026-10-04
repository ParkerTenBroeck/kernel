pub mod clint;


pub trait Timer{
    fn read_time(&self) -> u128;
}

pub trait Alarm: Timer{
    fn read_timeout(&self);
    fn set_timeout(&self, timeout: u128);
    fn add_timeout(&self, timeout: u128);
}

pub trait InterruptHandler {
    fn irq_handler(&mut self, irq: u32);
}

pub trait InterruptController: InterruptHandler{
    
}