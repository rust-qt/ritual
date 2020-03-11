use crate::ops::{Begin, BeginMut, End, EndMut, Increment, Indirection};
use crate::vector_ops::{Data, DataMut, Size};
use crate::{
    cpp_iter, CppBox, CppDeletable, CppIterator, DynamicCast, Ptr, StaticDowncast, StaticUpcast,
};
use std::ops::Deref;
use std::{fmt, ptr, slice};

/// A non-null, mutable pointer to a C++ object (similar to a C++ reference).
///
/// `Ref` never owns its content.
///
/// Note that unlike Rust references, `Ref` can be freely copied,
/// producing multiple mutable pointers to the same object, which is usually necessary
/// to do when working with C++ libraries.
///
/// `Ref` implements operator traits and delegates them
/// to the corresponding C++ operators.
/// This means that you can use `&ptr + value` to access the object's `operator+`.
///
/// `Ref` implements `Deref` allowing to call the object's methods
/// directly. In addition, methods of the object's first base class are also directly available
/// thanks to nested `Deref` implementations.
///
/// If the object provides an iterator interface through `begin()` and `end()` functions,
/// `Ref` will implement `IntoIterator`, so you can iterate on it directly.
///
/// ### Safety
///
/// It's not possible to automatically track the ownership of objects possibly managed by C++
/// libraries. The user must ensure that the object is alive while `Ref` exists. Note that
/// with `Ref`, it's possible to call unsafe C++ code without using any more unsafe Rust code,
/// for example, by using operator traits, so care should be taken when exposing
/// `Ref` in a safe interface.
pub struct Ref<T>(ptr::NonNull<T>);

/// Creates another pointer to the same object.
impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Ref(self.0)
    }
}

/// Creates another pointer to the same object.
impl<T> Copy for Ref<T> {}

impl<T> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ref({:?})", self.0)
    }
}

impl<T> Ref<T> {
    /// Creates a `Ref` from a `Ptr`. Returns `None` if `ptr` is null.
    ///
    /// ### Safety
    ///
    /// `ptr` must be valid. See type level documentation.
    pub unsafe fn new(ptr: Ptr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_raw_ptr())
    }

    /// Creates a `Ref` from a raw pointer. Returns `None` if `ptr` is null.
    ///
    /// ### Safety
    ///
    /// `ptr` must be valid. See type level documentation.
    pub unsafe fn from_raw(ptr: *const T) -> Option<Self> {
        ptr::NonNull::new(ptr as *mut T).map(Ref)
    }

    /// Creates a `Ref` from a non-null pointer.
    ///
    /// ### Safety
    ///
    /// `ptr` must be valid. See type level documentation.
    pub unsafe fn from_raw_non_null(ptr: ptr::NonNull<T>) -> Self {
        Ref(ptr)
    }

    /// Converts `self` to a `Ptr`.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. See type level documentation.
    pub unsafe fn as_ptr(self) -> Ptr<T> {
        Ptr::from_raw(self.as_raw_ptr())
    }

    /// Returns a reference to the value.
    ///
    /// ### Safety
    ///
    /// `self` must be valid.
    /// The content must not be read or modified through other ways while the returned reference
    /// exists.See type level documentation.
    pub unsafe fn as_raw_ref<'a>(self) -> &'a T {
        &*self.0.as_ptr()
    }

    /// Returns a mutable reference to the value.
    ///
    /// ### Safety
    ///
    /// `self` must be valid.
    /// The content must not be read or modified through other ways while the returned reference
    /// exists.See type level documentation.
    pub unsafe fn as_mut_raw_ref<'a>(self) -> &'a mut T {
        &mut *self.0.as_ptr()
    }

    /// Returns constant raw pointer to the value.
    pub fn as_raw_ptr(self) -> *const T {
        self.0.as_ptr()
    }

    /// Returns constant raw pointer to the value.
    pub fn as_mut_raw_ptr(self) -> *mut T {
        self.0.as_ptr()
    }

    /// Converts the pointer to the base class type `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn static_upcast<U>(self) -> Ref<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self.as_ptr())
            .as_ref()
            .expect("StaticUpcast returned null on Ref input")
    }

    /// Converts the pointer to the derived class type `U`.
    ///
    /// It's recommended to use `dynamic_cast` instead because it performs a checked conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid and it's type is `U` or inherits from `U`.
    pub unsafe fn static_downcast<U>(self) -> Ref<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self.as_ptr())
            .as_ref()
            .expect("StaticDowncast returned null on Ref input")
    }

    /// Converts the pointer to the derived class type `U`. Returns `None` if the object's type
    /// is not `U` and doesn't inherit `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn dynamic_cast<U>(self) -> Option<Ref<U>>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self.as_ptr()).as_ref()
    }
}

impl<V, T> Ref<V>
where
    V: Data<Output = *const T> + Size,
{
    /// Returns the content of the object as a slice, based on `data()` and `size()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn as_slice<'a>(self) -> &'a [T] {
        let ptr = self.data();
        let size = self.size();
        slice::from_raw_parts(ptr, size)
    }
}

impl<V, T> Ref<V>
where
    V: DataMut<Output = *mut T> + Size,
{
    /// Returns the content of the vector as a mutable slice,
    /// based on `data()` and `size()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn as_mut_slice<'a>(self) -> &'a mut [T] {
        let ptr = self.data_mut();
        let size = self.size();
        slice::from_raw_parts_mut(ptr, size)
    }
}

impl<T, T1, T2> Ref<T>
where
    T: Begin<Output = CppBox<T1>> + End<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Increment + Indirection,
    T2: CppDeletable,
{
    /// Returns an iterator over the content of the object,
    /// based on `begin()` and `end()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn iter(self) -> CppIterator<T1, T2> {
        cpp_iter(self.begin(), self.end())
    }
}

impl<T, T1, T2> Ref<T>
where
    T: BeginMut<Output = CppBox<T1>> + EndMut<Output = CppBox<T2>>,
    T1: CppDeletable + PartialEq<Ref<T2>> + Increment + Indirection,
    T2: CppDeletable,
{
    /// Returns a mutable iterator over the content of the object,
    /// based on `begin()` and `end()` methods.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    pub unsafe fn iter_mut(self) -> CppIterator<T1, T2> {
        cpp_iter(self.begin_mut(), self.end_mut())
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}
