use core::{panic::PanicInfo, sync::atomic::AtomicBool};

use crate::{arch, print, syscon};

static PANICKED: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::std::stdio::set_sout(|str| {
        crate::uart::uart().write_str(str);
    });
    if PANICKED.swap(true, core::sync::atomic::Ordering::SeqCst) {
        arch::halt()
    }
    print!("\nKERNEL PANIC: {info}");
    // syscon::poweroff();
    arch::halt()
}
