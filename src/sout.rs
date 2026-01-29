use core::{fmt::Write};

use crate::uart;


#[derive(Clone, Copy)]
pub struct Sout(fn(&str));
impl Write for Sout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        (self.0)(s);
        Ok(())
    }
}

pub static mut SOUT: Sout = Sout(uart::puts);

/// .
///
/// # Safety
///
/// .
pub unsafe fn set_sout(out: fn(&str)){
    unsafe{
        SOUT = Sout(out);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        write!(unsafe {$crate::sout::SOUT}, $($arg)*).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { {$crate::print!($($arg)*); $crate::println!(); }};
}