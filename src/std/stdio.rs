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


#[macro_export]
macro_rules! dbg {
    () => {
        $crate::println!("[{}:{}:{}]", $crate::file!(), $crate::line!(), $crate::column!())
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                $crate::println!("[{}:{}:{}] {} = {:#?}",
                    core::file!(),
                    core::line!(),
                    core::column!(),
                    core::stringify!($val),
                    &&tmp as &dyn core::fmt::Debug,
                );
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($($crate::dbg!($val)),+,)
    };
}