use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct RawSpinLock {
    lock: AtomicBool,
}

impl RawSpinLock {
    pub const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) {
        while self.lock.swap(true, Ordering::Acquire) {}
    }

    /// # Safety
    ///
    /// You must own the lock for this mutex to unlock it
    pub unsafe fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

impl Default for RawSpinLock {
    fn default() -> Self {
        Self::new()
    }
}

pub use critical::*;

mod critical {
    use super::*;

    pub struct CriticalSpinLockGuard<'a, T: ?Sized + 'a> {
        lock: &'a CriticalSpinLock<T>,
        ie: bool,
    }

    unsafe impl<T: ?Sized + Send> Send for CriticalSpinLock<T> {}
    unsafe impl<T: ?Sized + Send> Sync for CriticalSpinLock<T> {}

    impl<'a, T: ?Sized + 'a> Deref for CriticalSpinLockGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.inner.get() }
        }
    }

    impl<'a, T: ?Sized + 'a> DerefMut for CriticalSpinLockGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { &mut *self.lock.inner.get() }
        }
    }

    impl<'a, T: ?Sized + 'a> Drop for CriticalSpinLockGuard<'a, T> {
        fn drop(&mut self) {
            unsafe {
                self.lock.lock.unlock();
                if self.ie {
                    riscv::register::sstatus::set_spie();
                }
            }
        }
    }

    pub struct CriticalSpinLock<T: ?Sized> {
        lock: RawSpinLock,
        inner: UnsafeCell<T>,
    }

    impl<T> CriticalSpinLock<T> {
        pub const fn new(value: T) -> Self {
            Self {
                lock: RawSpinLock::new(),
                inner: UnsafeCell::new(value),
            }
        }
    }

    impl<T: ?Sized> CriticalSpinLock<T> {
        pub fn lock(&self) -> CriticalSpinLockGuard<'_, T> {
            let ie = riscv::register::sstatus::read().sie();
            unsafe {
                riscv::register::sstatus::clear_sie();
            }
            self.lock.lock();
            CriticalSpinLockGuard { lock: self, ie }
        }
    }
}
