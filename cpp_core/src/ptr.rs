use crate::ops::{Begin, BeginMut, End, EndMut, Increment, Indirection};
use crate::vector_ops::{Data, DataMut, Size};
use crate::{
    cpp_iter, CppBox, CppDeletable, CppIterator, DynamicCast, Ref, StaticDowncast, StaticUpcast,
};
use std::ops::Deref;
use std::{fmt, slice};

/// A pointer to a C++ object (similar to a C++ pointer).
///
/// A `Ptr` may or may not be owned. If you actually own the object, it's recommended to
/// convert it to `CppBox` using `to_box` method.
///
/// Note that unlike Rust references, `Ptr` can be freely copied,
/// producing multiple pointers to the same object, which is usually necessary
/// to do when working with C++ libraries.
///
/// `Ptr` implements operator traits and delegates them
/// to the corresponding C++ operators.
/// This means that you can use `&ptr + value` to access the object's `operator+`.
///
/// `Ptr` implements `Deref`, allowing to call the object's methods
/// directly. In addition, methods of the object's first base class are also directly available
/// thanks to nested `Deref` implementations.
///
/// `Ptr` can contain a null pointer. `Deref` will panic if attempted to dereference
/// a null pointer.
///
/// If the object provides an iterator interface through `begin()` and `end()` functions,
/// `Ptr` will implement `IntoIterator`, so you can iterate on it directly.
///
/// ### Safety
///
/// It's not possible to automatically track the ownership of objects possibly managed by C++
/// libraries. The user must ensure that the object is alive while `Ptr` exists. Note that
/// with `Ptr`, it's possible to call unsafe C++ code without using any more unsafe Rust code,
/// for example, by using operator traits, so care should be taken when exposing
/// `Ptr` in a safe interface.
///
/// Null pointers must not be dereferenced.
pub struct Ptr<T>(*mut T);

/// Creates another pointer to the same object.
impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Ptr(self.0)
    }
}

/// Creates another pointer to the same object.
impl<T> Copy for Ptr<T> {}

impl<T> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ptr({:?})", self.0)
    }
}

impl<T> Ptr<T> {
    /// Creates a `Ptr` from a raw pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        Ptr(ptr as *mut T)
    }

    /// Creates a null pointer.
    ///
    /// Note that accessing the content of a null `Ptr` through `Deref`
    /// will result in a panic.
    ///
    /// Note that you can also use `NullPtr` to specify a null pointer to a function accepting
    /// `impl CastInto<Ptr<_>>`. Unlike `Ptr`, `NullPtr` is not a generic type, so it will
    /// not cause type inference issues.
    ///
    /// ### Safety
    ///
    /// Null pointers must not be dereferenced. See type level documentation.
    pub unsafe fn null() -> Self {
        Ptr(std::ptr::null_mut())
    }

    /// Returns the content as a raw const pointer.
    pub fn as_mut_raw_ptr(self) -> *mut T {
        self.0 as *mut T
    }

    /// Returns the content as a raw const pointer.
    pub fn as_raw_ptr(self) -> *const T {
        self.0
    }

    /// Returns the content as a const `Ref`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// The operation is safe as long as `self` is valid or null. See type level documentation.
    pub unsafe fn as_ref(self) -> Option<Ref<T>> {
        Ref::from_raw(self.0)
    }

    /// Returns a reference to the value. Returns `None` if the pointer is null.
    ///
    /// ### Safety
    ///
    /// `self` must be valid.
    /// The content must not be read or modified through other ways while the returned reference
    /// exists.See type level documentation.
    pub unsafe fn as_raw_ref<'a>(self) -> Option<&'a T> {
        self.as_ref().map(|r| r.as_raw_ref())
    }

    /// Returns a mutable reference to the value. Returns `None` if the pointer is null.
    ///
    /// ### Safety
    ///
    /// `self` must be valid.
    /// The content must not be read or modified through other ways while the returned reference
    /// exists.See type level documentation.
    pub unsafe fn as_mut_raw_ref<'a>(self) -> Option<&'a mut T> {
        self.as_ref().map(|r| r.as_mut_raw_ref())
    }

    /// Returns true if the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Converts the pointer to the base class type `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid or null.
    pub unsafe fn static_upcast<U>(self) -> Ptr<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast(self)
    }

    /// Converts the pointer to the derived class type `U`.
    ///
    /// It's recommended to use `dynamic_cast` instead because it performs a checked conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid and it's type is `U` or inherits from `U`,
    /// of if `self` is a null pointer.
    pub unsafe fn static_downcast<U>(self) -> Ptr<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast(self)
    }

    /// Converts the pointer to the derived class type `U`. Returns `None` if the object's type
    /// is not `U` and doesn't inherit `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid or null.
    pub unsafe fn dynamic_cast<U>(self) -> Ptr<U>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast(self)
    }
}

impl<V, T> Ptr<V>
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

impl<V, T> Ptr<V>
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

impl<T, T1, T2> Ptr<T>
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

impl<T, T1, T2> Ptr<T>
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
///
/// Panics if the pointer is null.
impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if self.0.is_null() {
            panic!("attempted to deref a null Ptr<T>");
        }
        unsafe { &(*self.0) }
    }
}

impl<T: CppDeletable> Ptr<T> {
    /// Converts this pointer to a `CppBox`. Returns `None` if `self`
    /// is a null pointer.
    ///
    /// Use this function to take ownership of the object. This is
    /// the same as `CppBox::new`.
    ///
    /// # Safety
    ///
    /// See type level documentation. See also `CppBox::new` documentation.
    pub unsafe fn to_box(self) -> Option<CppBox<T>> {
        CppBox::new(self)
    }
}

/// A null pointer.
///
/// `NullPtr` implements `CastInto<Ptr<T>>`, so it can be
/// passed as argument to functions accepting pointers. It's possible to use `Ptr::null()`
/// as well, but that would require a type annotation.
pub struct NullPtr;

#[test]
fn ptr_deref() {
    let i = 42;
    unsafe {
        let ptr: Ptr<i32> = Ptr::from_raw(&i);
        assert_eq!(*ptr, 42);
    }
}
