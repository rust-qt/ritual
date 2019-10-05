use crate::ptr::NullPtr;
use crate::{CppBox, CppDeletable, MutPtr, MutRef, Ptr, Ref, StaticUpcast};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Performs some of the conversions that are available implicitly in C++.
///
/// `CastInto` is automatically implemented for all `CastFrom` conversions,
/// similar to `From` and `Into` traits from `std`.
pub trait CastFrom<T>: Sized {
    /// Performs the conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `value` is valid.
    unsafe fn cast_from(value: T) -> Self;
}

/// Performs some of the conversions that are available implicitly in C++.
///
/// `CastInto` is automatically implemented for all `CastFrom` conversions,
/// similar to `From` and `Into` traits from `std`.
pub trait CastInto<T>: Sized {
    /// Performs the conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    unsafe fn cast_into(self) -> T;
}

impl<T, U: CastFrom<T>> CastInto<U> for T {
    unsafe fn cast_into(self) -> U {
        U::cast_from(self)
    }
}

impl<T, U> CastFrom<MutPtr<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutPtr<U>) -> Self {
        <U as StaticUpcast<T>>::static_upcast_mut(value).as_ptr()
    }
}

impl<T, U> CastFrom<MutRef<U>> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
    }
}

impl<T, U> CastFrom<Ref<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: Ref<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
    }
}

impl<T, U> CastFrom<MutRef<U>> for Ref<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }
}

impl<T, U> CastFrom<MutRef<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a CppBox<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a CppBox<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a mut CppBox<U>> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a mut CppBox<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a CppBox<U>> for Ref<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a CppBox<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on CppBox input")
    }
}

impl<'a, T, U: CppDeletable> CastFrom<&'a mut CppBox<U>> for MutRef<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: &'a mut CppBox<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticUpcast returned null on CppBox input")
    }
}

impl<T, U> CastFrom<Ptr<U>> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: Ptr<U>) -> Self {
        StaticUpcast::static_upcast(value)
    }
}

impl<T, U> CastFrom<MutPtr<U>> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutPtr<U>) -> Self {
        StaticUpcast::static_upcast_mut(value)
    }
}

impl<T, U> CastFrom<Ref<U>> for Ref<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: Ref<U>) -> Self {
        StaticUpcast::static_upcast(value.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }
}

impl<T, U> CastFrom<MutRef<U>> for MutRef<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: MutRef<U>) -> Self {
        StaticUpcast::static_upcast_mut(value.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticUpcast returned null on Ref input")
    }
}

impl<T> CastFrom<NullPtr> for Ptr<T> {
    unsafe fn cast_from(_value: NullPtr) -> Self {
        Self::null()
    }
}

impl<T> CastFrom<NullPtr> for MutPtr<T> {
    unsafe fn cast_from(_value: NullPtr) -> Self {
        Self::null()
    }
}

impl<'a> CastFrom<&'a CStr> for MutPtr<c_char> {
    unsafe fn cast_from(value: &'a CStr) -> Self {
        MutPtr::from_c_str(value)
    }
}

impl<'a> CastFrom<&'a CStr> for Ptr<c_char> {
    unsafe fn cast_from(value: &'a CStr) -> Self {
        Ptr::from_c_str(value)
    }
}

impl<T, U> CastFrom<*const U> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: *const U) -> Self {
        Self::cast_from(Ptr::from_raw(value))
    }
}

impl<T, U> CastFrom<*mut U> for Ptr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: *mut U) -> Self {
        Self::cast_from(Ptr::from_raw(value as *const U))
    }
}

impl<T, U> CastFrom<*mut U> for MutPtr<T>
where
    U: StaticUpcast<T>,
{
    unsafe fn cast_from(value: *mut U) -> Self {
        Self::cast_from(MutPtr::from_raw(value))
    }
}
