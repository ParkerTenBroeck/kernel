pub mod sbi;

pub trait TimerDev {
    fn timebase_freq(&self) -> u64;
    fn time(&self) -> u64;
    fn set_deadline(&self, deadline: u64);
    fn set_enable(&self, enabled: bool);
    fn get_enabled(&self) -> bool;
}