use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::mem::Pointer;

pub struct PageVec<T> {
    len: usize,
    capacity: usize,
    data: NonNull<T>,
}

unsafe impl<T: Send> Send for PageVec<T> {}
unsafe impl<T: Sync> Sync for PageVec<T> {}

impl<T> Default for PageVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug> Debug for PageVec<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T> Deref for PageVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> DerefMut for PageVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl<T: Clone> Clone for PageVec<T> {
    fn clone(&self) -> Self {
        let mut clone = PageVec::new();
        clone.ensure_capacity(self.capacity());
        for element in self {
            clone.push(element.clone());
        }
        clone
    }
}

impl<'a, T> IntoIterator for &'a PageVec<T> {
    type Item = &'a T;

    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, T> IntoIterator for &'a mut PageVec<T> {
    type Item = &'a mut T;

    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice_mut().iter_mut()
    }
}

impl<T> PageVec<T> {
    #[track_caller]
    pub const fn new() -> Self {
        const {
            assert!(core::mem::align_of::<T>() < 4096);
        }
        Self {
            len: 0,
            capacity: 0,
            data: core::ptr::NonNull::dangling(),
        }
    }

    #[track_caller]
    pub fn push(&mut self, element: T) {
        self.ensure_capacity(self.len + 1);
        unsafe {
            self.data.add(self.len).write(element);
        }
    }

    #[track_caller]
    pub fn pop(&mut self) -> Option<T> {
        if !self.is_empty() {
            self.len -= 1;
            let last = unsafe { self.data.add(self.len).read() };
            Some(last)
        } else {
            None
        }
    }

    #[track_caller]
    pub fn insert(&mut self, index: usize, element: T) {
        if index > self.len {
            panic!("Invalid index must be <= len")
        }
        self.ensure_capacity(self.len + 1);

        unsafe {
            let p = self.data.as_ptr().add(index);
            core::ptr::copy(p, p.add(1), self.len - index);
            p.write(element);
            self.len += 1;
        }
    }

    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("Invalid index must be < len")
        }
        // crate::alloc::vec::Vec
        let element;
        unsafe {
            let p = self.data.as_ptr().add(index);
            element = p.read();
            core::ptr::copy(p.add(1), p, self.len - index - 1);
            self.len -= 1;
        }
        element
    }

    #[track_caller]
    pub fn ensure_capacity(&mut self, capacity: usize) {
        let capacity = capacity.next_power_of_two();
        if capacity > self.capacity {
            if self.capacity != 0{
                crate::println!("resized");
            }
            unsafe {
                let pages = crate::mem::pages::pages_zeroed(capacity.div_ceil(1 << 12)).virt();
                let Some(pages) = core::ptr::NonNull::new(pages) else {
                    panic!("OOM")
                };
                if self.data.as_ptr() != core::ptr::dangling_mut() {
                    pages
                        .as_ptr()
                        .copy_from(self.data.as_ptr().cast(), self.capacity.div_ceil(1 << 12));

                    crate::mem::pages::free_pages_contiguous(
                        Pointer::from_virt(self.data.as_ptr().cast()),
                        self.capacity.div_ceil(1 << 12),
                    );
                }

                self.data = pages.cast();
                self.capacity = capacity.div_ceil(1 << 12) * (1 << 12);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.data.as_ptr(), self.len) }
    }
}
