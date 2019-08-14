use crate::ops::{Begin, BeginMut, End, EndMut};
use crate::{DynamicCast, MutPtr, Ptr, StaticDowncast, StaticUpcast};
use std::ops::{Deref, DerefMut};
use std::{fmt, ptr};

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
        Self::from_raw(ptr.as_mut_raw_ptr())
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

    pub unsafe fn as_ptr(self) -> Ptr<T> {
        Ptr::from_raw(self.as_raw_ptr())
    }

    pub unsafe fn as_mut_ptr(self) -> MutPtr<T> {
        MutPtr::from_raw(self.as_mut_raw_ptr())
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *const T {
        self.0.as_ptr()
    }

    pub fn as_mut_raw_ptr(self) -> *mut T {
        self.0.as_ptr()
    }

    pub unsafe fn static_upcast<U>(self) -> Ref<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }

    pub unsafe fn static_downcast<U>(self) -> Ref<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self.as_ptr())
            .as_ref()
            .expect("StaticDowncast returned null on Ref input")
    }

    pub unsafe fn dynamic_cast<U>(self) -> Option<Ref<U>>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self.as_ptr()).as_ref()
    }

    pub unsafe fn static_upcast_mut<U>(self) -> MutRef<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast_mut(self.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticUpcast returned null on Ref input")
    }

    pub unsafe fn static_downcast_mut<U>(self) -> MutRef<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast_mut(self.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticDowncast returned null on Ref input")
    }

    pub unsafe fn dynamic_cast_mut<U>(self) -> Option<MutRef<U>>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast_mut(self.as_mut_ptr()).as_mut_ref()
    }

    pub unsafe fn begin(self) -> <&'static T as Begin>::Output
    where
        for<'a> &'a T: Begin,
    {
        (*self.as_raw_ptr()).begin()
    }

    pub unsafe fn begin_mut(self) -> <&'static mut T as BeginMut>::Output
    where
        for<'a> &'a mut T: BeginMut,
    {
        (*self.as_mut_raw_ptr()).begin_mut()
    }

    pub unsafe fn end(self) -> <&'static T as End>::Output
    where
        for<'a> &'a T: End,
    {
        (*self.as_raw_ptr()).end()
    }

    pub unsafe fn end_mut(self) -> <&'static mut T as EndMut>::Output
    where
        for<'a> &'a mut T: EndMut,
    {
        (*self.as_mut_raw_ptr()).end_mut()
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

    pub unsafe fn as_ptr(self) -> Ptr<T> {
        Ptr::from_raw(self.as_raw_ptr())
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_raw_ptr(self) -> *const T {
        self.0.as_ptr()
    }

    pub unsafe fn static_upcast<U>(self) -> Ref<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }

    pub unsafe fn static_downcast<U>(self) -> Ref<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self.as_ptr())
            .as_ref()
            .expect("StaticDowncast returned null on Ref input")
    }

    pub unsafe fn dynamic_cast<U>(self) -> Option<Ref<U>>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self.as_ptr()).as_ref()
    }

    pub unsafe fn begin(self) -> <&'static T as Begin>::Output
    where
        for<'a> &'a T: Begin,
    {
        (*self.as_raw_ptr()).begin()
    }

    pub unsafe fn end(self) -> <&'static T as End>::Output
    where
        for<'a> &'a T: End,
    {
        (*self.as_raw_ptr()).end()
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
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
