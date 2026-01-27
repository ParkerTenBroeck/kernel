use core::{fmt::Write};

use crate::uart;


#[derive(Clone, Copy)]
pub struct Sout(fn(&[u8]));
impl Write for Sout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        (self.0)(s.as_bytes());
        Ok(())
    }
}

pub static mut SOUT: Sout = Sout(uart::putb);

/// .
///
/// # Safety
///
/// .
pub unsafe fn set_sout(out: fn(&[u8])){
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