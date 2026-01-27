use core::{panic::PanicInfo, sync::atomic::AtomicBool};

use crate::{arch, print, syscon};

static PANICKED: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if PANICKED.swap(true, core::sync::atomic::Ordering::SeqCst){
        loop{
            arch::wfi();
        }
    }
    print!("\nKERNEL PANIC: {info}");
    syscon::poweroff();
}