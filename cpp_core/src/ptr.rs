use crate::{CppBox, CppDeletable, DynamicCast, MutRef, Ref, StaticDowncast, StaticUpcast};
use std::ffi::CStr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;

/// A mutable pointer to a C++ object (similar to a C++ pointer).
///
/// A `MutPtr` may or may not be owned. If you actually own the object, it's recommended to
/// convert it to `CppBox` using `to_box` method.
///
/// Note that unlike Rust references, `MutPtr` can be freely copied,
/// producing multiple mutable pointers to the same object, which is usually necessary
/// to do when working with C++ libraries.
///
/// `MutPtr` implements operator traits and delegates them
/// to the corresponding C++ operators.
/// This means that you can use `&ptr + value` to access the object's `operator+`.
///
/// `MutPtr` implements `Deref` and `DerefMut`, allowing to call the object's methods
/// directly. In addition, methods of the object's first base class are also directly available
/// thanks to nested `Deref` implementations.
///
/// `MutPtr` can contain a null pointer. `Deref` and `DerefMut`
/// will panic if attempted to dereference a null pointer.
///
/// If the object provides an iterator interface through `begin()` and `end()` functions,
/// `MutPtr` will implement `IntoIterator`, so you can iterate on it directly.
///
/// ### Safety
///
/// It's not possible to automatically track the ownership of objects possibly managed by C++
/// libraries. The user must ensure that the object is alive while `MutPtr` exists. Note that
/// with `MutPtr`, it's possible to call unsafe C++ code without using any more unsafe Rust code,
/// for example, by using operator traits, so care should be taken when exposing
/// `MutPtr` in a safe interface.
///
/// Null pointers must not be dereferenced.
pub struct MutPtr<T>(*mut T);

/// Creates another pointer to the same object.
impl<T> Clone for MutPtr<T> {
    fn clone(&self) -> Self {
        MutPtr(self.0)
    }
}

/// Creates another pointer to the same object.
impl<T> Copy for MutPtr<T> {}

impl<T> fmt::Debug for MutPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutPtr({:?})", self.0)
    }
}

impl<T> MutPtr<T> {
    /// Creates a `MutPtr` from a raw pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        MutPtr(ptr)
    }

    /// Creates a null pointer.
    ///
    /// Note that accessing the content of a null `MutPtr` through `Deref` or `DerefMut`
    /// will result in a panic.
    ///
    /// Note that you can also use `NullPtr` to specify a null pointer to a function accepting
    /// `impl CastInto<MutPtr<_>>`. Unlike `MutPtr`, `NullPtr` is not a generic type, so it will
    /// not cause type inference issues.
    ///
    /// ### Safety
    ///
    /// Null pointers must not be dereferenced. See type level documentation.
    pub unsafe fn null() -> Self {
        MutPtr(std::ptr::null_mut())
    }

    /// Returns the content as a raw const pointer.
    pub fn as_raw_ptr(self) -> *const T {
        self.0
    }

    /// Returns the content as a raw mutable pointer.
    pub fn as_mut_raw_ptr(self) -> *mut T {
        self.0
    }

    /// Returns the content as a const `Ptr`.
    ///
    /// ### Safety
    ///
    /// The operation is safe as long as `self` is valid or null. See type level documentation.
    pub unsafe fn as_ptr(self) -> Ptr<T> {
        Ptr::from_raw(self.0)
    }

    /// Returns the content as a const `Ref`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// The operation is safe as long as `self` is valid or null. See type level documentation.
    pub unsafe fn as_ref(self) -> Option<Ref<T>> {
        Ref::from_raw(self.0)
    }

    /// Returns the content as a `MutRef`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// The operation is safe as long as `self` is valid or null. See type level documentation.
    pub unsafe fn as_mut_ref(self) -> Option<MutRef<T>> {
        MutRef::from_raw(self.0)
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
        StaticUpcast::static_upcast(self.as_ptr())
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
        StaticDowncast::static_downcast(self.as_ptr())
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
        DynamicCast::dynamic_cast(self.as_ptr())
    }

    /// Converts the pointer to the base class type `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid or null.
    pub unsafe fn static_upcast_mut<U>(self) -> MutPtr<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast_mut(self)
    }

    /// Converts the pointer to the derived class type `U`.
    ///
    /// It's recommended to use `dynamic_cast` instead because it performs a checked conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid and it's type is `U` or inherits from `U`,
    /// of if `self` is a null pointer.
    pub unsafe fn static_downcast_mut<U>(self) -> MutPtr<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast_mut(self)
    }

    /// Converts the pointer to the derived class type `U`. Returns `None` if the object's type
    /// is not `U` and doesn't inherit `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid or null.
    pub unsafe fn dynamic_cast_mut<U>(self) -> MutPtr<U>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast_mut(self)
    }
}

impl<T: CppDeletable> MutPtr<T> {
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

/// Allows to call member functions of `T` and its base classes directly on the pointer.
///
/// Panics if the pointer is null.
impl<T> Deref for MutPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        unsafe { &(*self.0) }
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
///
/// Panics if the pointer is null.
impl<T> DerefMut for MutPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        if self.0.is_null() {
            panic!("attempted to deref a null MutPtr<T>");
        }
        unsafe { &mut (*self.0) }
    }
}

impl MutPtr<c_char> {
    /// Creates a `MutPtr<c_char>`, i.e. C++'s `char*` from a `CStr`.
    ///
    /// ### Safety
    ///
    /// The source `str` must be valid
    /// while `MutPtr` exists and while
    /// it's used by the C++ library.
    ///
    /// After passing `str` to `MutPtr`, it's unsafe to use `str` and
    /// any references to the same buffer from Rust because
    /// the memory can be modified through `MutPtr`.
    pub unsafe fn from_c_str(str: &CStr) -> Self {
        Self::from_raw(str.as_ptr() as *mut c_char)
    }

    /// Converts `MutPtr<c_char>`, i.e. C++'s `char*` to a `&CStr`.
    ///
    /// ### Safety
    ///
    /// No guarantees can be made about the validity and lifetime of
    /// the buffer, since it could be produced by a C++ library.
    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

/// A const pointer to a C++ object (similar to a C++ pointer).
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
pub struct Ptr<T>(*const T);

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
        Ptr(ptr)
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
        Ptr(std::ptr::null())
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

/// Allows to call member functions of `T` and its base classes directly on the pointer.
impl Ptr<c_char> {
    /// Creates a `Ptr<c_char>`, i.e. C++'s `const char*` from a `CStr`.
    ///
    /// ### Safety
    ///
    /// The source `str` must be valid
    /// while `Ptr` exists and while
    /// it's used by the C++ library.
    ///
    /// After passing `str` to `Ptr`, it's unsafe to use `str` and
    /// any references to the same buffer from Rust because
    /// the memory can be modified through `Ptr`.
    pub unsafe fn from_c_str(str: &CStr) -> Self {
        Self::from_raw(str.as_ptr())
    }

    /// Converts `Ptr<c_char>`, i.e. C++'s `const char*` to a `&CStr`.
    ///
    /// ### Safety
    ///
    /// No guarantees can be made about the validity and lifetime of
    /// the buffer, since it could be produced by a C++ library.
    pub unsafe fn to_c_str<'a>(self) -> &'a CStr {
        CStr::from_ptr(self.0)
    }
}

/// A null pointer.
///
/// `NullPtr` implements `CastInto<Ptr<T>>` and `CastInto<MutPtr<T>>`, so it can be
/// passed as argument to functions accepting pointers. It's possible to use `Ptr::null()`
/// as well, but that would require a type annotation.
pub struct NullPtr;

#[test]
fn ptr_deref() {
    let mut i = 42;
    unsafe {
        let ptr: MutPtr<i32> = MutPtr::from_raw(&mut i as *mut i32);
        assert_eq!(*ptr, 42);
    }
}
