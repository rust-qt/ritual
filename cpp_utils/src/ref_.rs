use crate::{ConstPtr, Ptr};
use std::ops::{Deref, DerefMut};
use std::{fmt, ptr};

#[derive(PartialEq, Eq)]
pub struct Ref<T>(ptr::NonNull<T>);

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Ref(self.0)
    }
}

impl<T> Copy for Ref<T> {}

impl<T> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ref({:?})", self.0)
    }
}

impl<T> Ref<T> {
    pub unsafe fn new(ptr: Ptr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        ptr::NonNull::new(ptr).map(Ref)
    }

    pub unsafe fn from_raw_non_null(ptr: ptr::NonNull<T>) -> Self {
        Ref(ptr)
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> DerefMut for Ref<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

pub struct ConstRef<T>(ptr::NonNull<T>);

impl<T> Clone for ConstRef<T> {
    fn clone(&self) -> Self {
        ConstRef(self.0)
    }
}

impl<T> Copy for ConstRef<T> {}

impl<T> fmt::Debug for ConstRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConstRef({:?})", self.0)
    }
}

impl<T> ConstRef<T> {
    pub unsafe fn new(ptr: ConstPtr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    pub unsafe fn from_raw(ptr: *const T) -> Option<Self> {
        ptr::NonNull::new(ptr as *mut T).map(ConstRef)
    }

    pub unsafe fn from_raw_non_null(ptr: ptr::NonNull<T>) -> Self {
        ConstRef(ptr)
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *const T {
        self.0.as_ptr()
    }
}

impl<T> Deref for ConstRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> From<Ref<T>> for ConstRef<T> {
    fn from(value: Ref<T>) -> Self {
        ConstRef(value.0)
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
