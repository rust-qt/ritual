use crate::{CppBox, CppDeletable, MutRef, Ref};
use std::ffi::CStr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;

#[derive(PartialEq, Eq)]
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

    /// Returns mutable raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *mut T {
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
    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

#[test]
fn ptr_deref() {
    let mut i = 42;
    unsafe {
        let ptr: MutPtr<i32> = MutPtr::from_raw(&mut i as *mut i32);
        assert_eq!(*ptr, 42);
    }
}
