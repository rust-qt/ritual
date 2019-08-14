use crate::ops::{Begin, BeginMut, End, EndMut};
use crate::{CppBox, CppDeletable, DynamicCast, MutRef, Ref, StaticDowncast, StaticUpcast};
use std::ffi::CStr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;

pub struct MutPtr<T>(*mut T);

impl<T> Clone for MutPtr<T> {
    fn clone(&self) -> Self {
        MutPtr(self.0)
    }
}

impl<T> Copy for MutPtr<T> {}

impl<T> fmt::Debug for MutPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutPtr({:?})", self.0)
    }
}

impl<T> MutPtr<T> {
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        MutPtr(ptr)
    }

    pub unsafe fn null() -> Self {
        MutPtr(std::ptr::null_mut())
    }

    pub fn as_raw_ptr(self) -> *const T {
        self.0
    }

    pub fn as_mut_raw_ptr(self) -> *mut T {
        self.0
    }

    pub unsafe fn as_ptr(self) -> Ptr<T> {
        Ptr::from_raw(self.0)
    }

    pub unsafe fn as_ref(self) -> Option<Ref<T>> {
        Ref::from_raw(self.0)
    }

    pub unsafe fn as_mut_ref(self) -> Option<MutRef<T>> {
        MutRef::from_raw(self.0)
    }

    /// Returns true if the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    pub unsafe fn static_upcast<U>(self) -> Ptr<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self.as_ptr())
    }

    pub unsafe fn static_downcast<U>(self) -> Ptr<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self.as_ptr())
    }

    pub unsafe fn dynamic_cast<U>(self) -> Ptr<U>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self.as_ptr())
    }

    pub unsafe fn static_upcast_mut<U>(self) -> MutPtr<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast_mut(self)
    }

    pub unsafe fn static_downcast_mut<U>(self) -> MutPtr<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast_mut(self)
    }

    pub unsafe fn dynamic_cast_mut<U>(self) -> MutPtr<U>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast_mut(self)
    }

    pub unsafe fn begin(self) -> <&'static T as Begin>::Output
    where
        for<'a> &'a T: Begin,
    {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        (*self.as_raw_ptr()).begin()
    }

    pub unsafe fn begin_mut(self) -> <&'static mut T as BeginMut>::Output
    where
        for<'a> &'a mut T: BeginMut,
    {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        (*self.as_mut_raw_ptr()).begin_mut()
    }

    pub unsafe fn end(self) -> <&'static T as End>::Output
    where
        for<'a> &'a T: End,
    {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        (*self.as_raw_ptr()).end()
    }

    pub unsafe fn end_mut(self) -> <&'static mut T as EndMut>::Output
    where
        for<'a> &'a mut T: EndMut,
    {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        (*self.as_mut_raw_ptr()).end_mut()
    }
}

impl<T: CppDeletable> MutPtr<T> {
    pub unsafe fn to_box(self) -> Option<CppBox<T>> {
        CppBox::new(self)
    }
}

impl<T> Deref for MutPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        unsafe { &(*self.0) }
    }
}

impl<T> DerefMut for MutPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        unsafe { &mut (*self.0) }
    }
}

impl MutPtr<c_char> {
    pub unsafe fn from_c_str(str: &CStr) -> Self {
        Self::from_raw(str.as_ptr() as *mut c_char)
    }

    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

pub struct Ptr<T>(*const T);

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Ptr(self.0)
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ptr({:?})", self.0)
    }
}

impl<T> Ptr<T> {
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        Ptr(ptr)
    }

    pub unsafe fn null() -> Self {
        Ptr(std::ptr::null())
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *const T {
        self.0
    }

    pub unsafe fn as_ref(self) -> Option<Ref<T>> {
        Ref::from_raw(self.0)
    }

    /// Returns true if the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    pub unsafe fn static_upcast<U>(self) -> Ptr<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self)
    }

    pub unsafe fn static_downcast<U>(self) -> Ptr<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self)
    }

    pub unsafe fn dynamic_cast<U>(self) -> Ptr<U>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self)
    }

    pub unsafe fn begin(self) -> <&'static T as Begin>::Output
    where
        for<'a> &'a T: Begin,
    {
        if self.0.is_null() {
            panic!("attempted to deref a null Ptr<T>");
        }
        (*self.as_raw_ptr()).begin()
    }

    pub unsafe fn end(self) -> <&'static T as End>::Output
    where
        for<'a> &'a T: End,
    {
        if self.0.is_null() {
            panic!("attempted to deref a null Ptr<T>");
        }
        (*self.as_raw_ptr()).end()
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if self.0.is_null() {
            panic!("attempted to deref a null Ptr<T>");
        }
        unsafe { &(*self.0) }
    }
}

impl Ptr<c_char> {
    pub unsafe fn from_c_str(str: &CStr) -> Self {
        Self::from_raw(str.as_ptr())
    }

    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

pub struct NullPtr;

#[test]
fn ptr_deref() {
    let mut i = 42;
    unsafe {
        let ptr: MutPtr<i32> = MutPtr::from_raw(&mut i as *mut i32);
        assert_eq!(*ptr, 42);
    }
}
