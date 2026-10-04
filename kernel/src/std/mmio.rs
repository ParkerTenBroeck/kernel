use core::{cell::UnsafeCell, mem::MaybeUninit};

pub trait UnsafeRead<T> {
    unsafe fn read(&self) -> T;
}

pub trait SafeRead<T> {
    fn read(&self) -> T;
}

pub trait UnsafeWrite<T> {
    unsafe fn write(&self, val: T);
}

pub trait SafeWrite<T> {
    fn write(&self, val: T);
}

macro_rules! mk_impl {
    (safe_read $meow:ident) => {
        impl<T: Copy> SafeRead<T> for $meow<T>{
            #[inline(always)]
            fn read(&self) -> T {
                unsafe { atomic_volatile_load(self.0.get().cast_const()) }
            }
        }
        impl<T: core::fmt::Debug + Copy> core::fmt::Debug for $meow<T> {
            fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> { 
                self.read().fmt(fmt)
            }
        }
    };
    (unsafe_read $meow:ident) => {
        impl<T: Copy> UnsafeRead<T> for $meow<T>{
            #[inline(always)]
            unsafe fn read(&self) -> T {
                unsafe { atomic_volatile_load(self.0.get().cast_const()) }
            }
        }
    };
    (safe_write $meow:ident) => {
        impl<T> SafeWrite<T> for $meow<T>{
            #[inline(always)]
            fn write(&self, val: T) {
                unsafe { atomic_volatile_store(self.0.get(), val) }
            }
        }
    };
    (unsafe_write $meow:ident) => {
        impl<T> UnsafeWrite<T> for $meow<T>{
            #[inline(always)]
            unsafe fn write(&self, val: T) {
                unsafe { atomic_volatile_store(self.0.get(), val) }
            }
        }
    };
    (debug $meow:ident) => {
        impl<T> core::fmt::Debug for $meow<T>{
            fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> { 
                fmt.debug_struct(core::any::type_name::<Self>()).finish()
            }
        }
    };
}


#[inline(always)]
unsafe fn atomic_volatile_load<T>(ptr: *const T) -> T {
    const {
        match (core::mem::size_of::<T>(), core::mem::align_of::<T>()) {
            (1,1)|(2,2)|(4,4)|(8,8) => {},
            _ => panic!()
        }
    }
    match (core::mem::size_of::<T>(), core::mem::align_of::<T>()) {
        (1, 1) => unsafe {
            let val = core::sync::atomic::AtomicU8::from_ptr(ptr.cast_mut().cast())
            .load(core::sync::atomic::Ordering::SeqCst);
            core::mem::transmute_copy::<u8, T>(&val)
        }
        (2, 2) => unsafe {
            let val = core::sync::atomic::AtomicU16::from_ptr(ptr.cast_mut().cast())
            .load(core::sync::atomic::Ordering::SeqCst);
            core::mem::transmute_copy::<u16, T>(&val)
        }
        (4, 4) => unsafe {
            let val = core::sync::atomic::AtomicU32::from_ptr(ptr.cast_mut().cast())
            .load(core::sync::atomic::Ordering::SeqCst);
            core::mem::transmute_copy::<u32, T>(&val)
        }
        (8, 8) => unsafe {
            let val = core::sync::atomic::AtomicU64::from_ptr(ptr.cast_mut().cast())
            .load(core::sync::atomic::Ordering::SeqCst);
            core::mem::transmute_copy::<u64, T>(&val)
        }
        _ => unreachable!()
    }
}


#[inline(always)]
unsafe fn atomic_volatile_store<T>(ptr: *mut T, val: T) {
    const {
        match (core::mem::size_of::<T>(), core::mem::align_of::<T>()) {
            (1,1)|(2,2)|(4,4)|(8,8) => {},
            _ => panic!()
        }
    }
    match (core::mem::size_of::<T>(), core::mem::align_of::<T>()) {
        (1, 1) => unsafe {
            let val = core::mem::transmute_copy::<T, _>(&val);
            core::sync::atomic::AtomicU8::from_ptr(ptr.cast())
            .store(val, core::sync::atomic::Ordering::SeqCst);
        }
        (2, 2) => unsafe {
            let val = core::mem::transmute_copy::<T, _>(&val);
            core::sync::atomic::AtomicU16::from_ptr(ptr.cast())
            .store(val, core::sync::atomic::Ordering::SeqCst);
        }
        (4, 4) => unsafe {
            let val = core::mem::transmute_copy::<T, _>(&val);
            core::sync::atomic::AtomicU32::from_ptr(ptr.cast())
            .store(val, core::sync::atomic::Ordering::SeqCst);
        }
        (8, 8) => unsafe {
            let val = core::mem::transmute_copy::<T, _>(&val);
            core::sync::atomic::AtomicU64::from_ptr(ptr.cast())
            .store(val, core::sync::atomic::Ordering::SeqCst);
        }
        _ => unreachable!()
    }
}

#[repr(transparent)]
pub struct P<T>(MaybeUninit<T>);
mk_impl!(debug P);

#[repr(transparent)]
pub struct R<T>(UnsafeCell<T>);
mk_impl!(unsafe_read R);
mk_impl!(debug R);

#[repr(transparent)]
pub struct SR<T>(UnsafeCell<T>);
mk_impl!(safe_read SR);

#[repr(transparent)]
pub struct W<T>(UnsafeCell<T>);
mk_impl!(unsafe_write W);
mk_impl!(debug W);


#[repr(transparent)]
pub struct SW<T>(UnsafeCell<T>);
mk_impl!(safe_write SW);
mk_impl!(debug SW);

#[repr(transparent)]
pub struct RW<T>(UnsafeCell<T>);
mk_impl!(unsafe_read RW);
mk_impl!(unsafe_write RW);
mk_impl!(debug RW);

#[repr(transparent)]
pub struct SRW<T>(UnsafeCell<T>);
mk_impl!(safe_read SRW);
mk_impl!(unsafe_write SRW);


#[repr(transparent)]
pub struct RSW<T>(UnsafeCell<T>);
mk_impl!(unsafe_read RSW);
mk_impl!(safe_write RSW);
mk_impl!(debug RSW);


#[repr(transparent)]
pub struct SRSW<T>(UnsafeCell<T>);
mk_impl!(safe_read SRSW);
mk_impl!(safe_write SRSW);



trait Endianess<T> {
    fn from(val: T) -> T;
    fn to(val: T) -> T;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LE;
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BE;
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NE;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Num<N, E>{
    value: N,
    _endianess: core::marker::PhantomData<E>,
}
impl<N: Copy, E: Endianess<N>> Num<N, E> {
    pub fn num(&self) -> N {
        E::from(self.value)
    }
}

macro_rules! mk_num_impl {
    ($ty:ident, $le:ident, $be:ident, $ne:ident) => {
        #[allow(non_camel_case_types)]
        pub type $le = Num<$ty, LE>;
        #[allow(non_camel_case_types)]
        pub type $be = Num<$ty, BE>;
        #[allow(non_camel_case_types)]
        pub type $ne = Num<$ty, NE>;

        impl Endianess<$ty> for LE {
            #[inline(always)]
            fn from(val: $ty) -> $ty { $ty::from_le(val) }
            #[inline(always)]
            fn to(val: $ty) -> $ty { val.to_le() }
        }

        impl Endianess<$ty> for BE {
            #[inline(always)]
            fn from(val: $ty) -> $ty { $ty::from_be(val) }
            #[inline(always)]
            fn to(val: $ty) -> $ty { val.to_be() }
        }

        impl Endianess<$ty> for NE {
            #[inline(always)]
            fn from(val: $ty) -> $ty { val }
            #[inline(always)]
            fn to(val: $ty) -> $ty { val }
        }

        impl<E: Endianess<$ty>> From<$ty> for Num<$ty, E> {
            #[inline(always)]
            fn from(value: $ty) -> Self {
                Self { value: E::to(value), _endianess: core::marker::PhantomData }
            }
        }

        impl<E: Endianess<$ty>> From<Num<$ty, E>> for $ty {
            #[inline(always)]
            fn from(value: Num<$ty, E>) -> Self {
                E::from(value.value)
            }
        }
        impl<E: Endianess<$ty>> From<&Num<$ty, E>> for $ty {
            #[inline(always)]
            fn from(value: &Num<$ty, E>) -> Self {
                E::from(value.value)
            }
        }

        impl<E: Endianess<$ty>> core::fmt::Display for Num<$ty, E> {
            fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> { 
                let num: $ty = self.into();
                core::fmt::Display::fmt(&num, fmt)
            }
        }

        impl<E: Endianess<$ty>> core::fmt::Debug for Num<$ty, E> {
            fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> { 
                let num: $ty = self.into();
                core::fmt::Debug::fmt(&num, fmt)
            }
        }
    };
}


mk_num_impl!(i8, i8le, i8be, i8ne);
mk_num_impl!(i16, i16le, i16be, i16ne);
mk_num_impl!(i32, i32le, i32be, i32ne);
mk_num_impl!(i64, i64le, i64be, i64ne);
mk_num_impl!(i128, i128le, i128be, i128ne);
mk_num_impl!(isize, isizele, isizebe, isizene);
mk_num_impl!(u8, u8le, u8be, u8ne);
mk_num_impl!(u16, u16le, u16be, u16ne);
mk_num_impl!(u32, u32le, u32be, u32ne);
mk_num_impl!(u64, u64le, u64be, u64ne);
mk_num_impl!(u128, u128le, u128be, u128ne);
mk_num_impl!(usize, usizele, usizebe, usizene);