use core::fmt::Write;

#[derive(Clone, Copy)]
pub struct Sout(fn(&str));
impl Write for Sout {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        (self.0)(s);
        Ok(())
    }
}

pub static mut SOUT: Sout = Sout(|_| {});

pub fn set_sout(out: fn(&str)) {
    unsafe {
        SOUT = Sout(out);
    }
}

pub fn sout() -> Sout {
    unsafe { SOUT }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        write!($crate::std::stdio::sout(), $($arg)*).unwrap();
    }};
}

#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { {$crate::print!($($arg)*); $crate::println!(); }};
}
