pub mod plic;

unsafe trait InterruptHandler{
    fn handle(&self);
}