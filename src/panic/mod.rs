use core::{fmt::Write, panic::PanicInfo, sync::atomic::AtomicBool};

use crate::{arch, print};

static PANICKED: AtomicBool = AtomicBool::new(false);

pub fn print_u32_dec(out: &mut crate::std::stdio::Sout, mut n: u32) {
    // Special case for 0
    if n == 0 {
        _ = out.write_char('0');
        return;
    }

    // u32 max is 10 decimal digits
    let mut buf = [0u8; 10];
    let mut i = 0;

    // Convert digits in reverse
    while n > 0 {
        buf[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
    }

    // Output in correct order
    while i > 0 {
        i -= 1;
        _ = out.write_char(buf[i] as char);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::std::stdio::set_sout(|str| {
        arch::entry::early_print(str);
    });
    if PANICKED.swap(true, core::sync::atomic::Ordering::SeqCst) {
        arch::halt()
    }
    let mut out = crate::std::stdio::sout();
    _ = out.write_str("\nKERNEL PANIC\n");
    print!("{info}");
    arch::halt()
}
