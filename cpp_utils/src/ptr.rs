use crate::{ConstRef, CppBox, CppDeletable, Ref};
use std::ffi::CStr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;

#[derive(PartialEq, Eq)]
pub struct Ptr<T>(*mut T);

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
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Ptr(ptr)
    }

    pub unsafe fn null() -> Self {
        Ptr(std::ptr::null_mut())
    }

    /// Returns mutable raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *mut T {
        self.0
    }

    pub unsafe fn as_ref(self) -> Option<ConstRef<T>> {
        ConstRef::from_raw(self.0)
    }

    pub unsafe fn as_mut_ref(self) -> Option<Ref<T>> {
        Ref::from_raw(self.0)
    }

    /// Returns true if the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<T: CppDeletable> Ptr<T> {
    pub unsafe fn to_box(self) -> Option<CppBox<T>> {
        CppBox::new(self)
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

impl<T> DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut T {
        if self.0.is_null() {
            panic!("attempted to deref a null Ptr<T>");
        }
        unsafe { &mut (*self.0) }
    }
}

impl Ptr<c_char> {
    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

pub struct ConstPtr<T>(*const T);

impl<T> Clone for ConstPtr<T> {
    fn clone(&self) -> Self {
        ConstPtr(self.0)
    }
}

impl<T> Copy for ConstPtr<T> {}

impl<T> fmt::Debug for ConstPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConstPtr({:?})", self.0)
    }
}

impl<T> ConstPtr<T> {
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        ConstPtr(ptr)
    }

    pub unsafe fn null() -> Self {
        ConstPtr(std::ptr::null())
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *const T {
        self.0
    }

    pub unsafe fn as_ref(self) -> Option<ConstRef<T>> {
        ConstRef::from_raw(self.0)
    }

    /// Returns true if the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<T> Deref for ConstPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if self.0.is_null() {
            panic!("attempted to deref a null ConstPtr<T>");
        }
        unsafe { &(*self.0) }
    }
}

impl<T> From<Ptr<T>> for ConstPtr<T> {
    fn from(value: Ptr<T>) -> Self {
        ConstPtr(value.0)
    }
}

impl<T> From<Ref<T>> for Ptr<T> {
    fn from(value: Ref<T>) -> Self {
        Ptr(value.as_raw_ptr())
    }
}

impl<T> From<ConstRef<T>> for ConstPtr<T> {
    fn from(value: ConstRef<T>) -> Self {
        ConstPtr(value.as_raw_ptr())
    }
}

impl ConstPtr<c_char> {
    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

#[test]
fn ptr_deref() {
    let mut i = 42;
    unsafe {
        let ptr: Ptr<i32> = Ptr::from_raw(&mut i as *mut i32);
        assert_eq!(*ptr, 42);
    }
}
