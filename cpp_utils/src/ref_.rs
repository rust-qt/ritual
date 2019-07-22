use crate::MutPtr;
use std::ops::{Deref, DerefMut};
use std::{fmt, ptr};

#[derive(PartialEq, Eq)]
pub struct MutRef<T>(ptr::NonNull<T>);

impl<T> Clone for MutRef<T> {
    fn clone(&self) -> Self {
        MutRef(self.0)
    }
}

impl<T> Copy for MutRef<T> {}

impl<T> fmt::Debug for MutRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutRef({:?})", self.0)
    }
}

impl<T> MutRef<T> {
    pub unsafe fn new(ptr: MutPtr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        ptr::NonNull::new(ptr).map(MutRef)
    }

    pub unsafe fn from_raw_ref(value: &mut T) -> Self {
        MutRef(value.into())
    }

    pub unsafe fn from_raw_non_null(ptr: ptr::NonNull<T>) -> Self {
        MutRef(ptr)
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *mut T {
        self.0.as_ptr()
    }
}

impl<T> Deref for MutRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> DerefMut for MutRef<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

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
    pub unsafe fn new(ptr: MutPtr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    pub unsafe fn from_raw(ptr: *const T) -> Option<Self> {
        ptr::NonNull::new(ptr as *mut T).map(Ref)
    }

    pub unsafe fn from_raw_ref(value: &T) -> Self {
        Ref(ptr::NonNull::new(value as *const T as *mut T).unwrap())
    }

    pub unsafe fn from_raw_non_null(ptr: ptr::NonNull<T>) -> Self {
        Ref(ptr)
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *const T {
        self.0.as_ptr()
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> From<MutRef<T>> for Ref<T> {
    fn from(value: MutRef<T>) -> Self {
        Ref(value.0)
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
