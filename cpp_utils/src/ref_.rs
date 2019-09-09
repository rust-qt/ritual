use crate::ops::{Begin, BeginMut, End, EndMut};
use crate::{DynamicCast, MutPtr, Ptr, StaticDowncast, StaticUpcast};
use std::ops::{Deref, DerefMut};
use std::{fmt, mem, ptr, slice};

/// A non-null, mutable pointer to a C++ object (similar to a C++ reference).
///
/// `MutRef` never owns its content.
///
/// Note that unlike Rust references, `MutRef` can be freely copied,
/// producing multiple mutable pointers to the same object, which is usually necessary
/// to do when working with C++ libraries.
///
/// `MutRef` implements operator traits and delegates them
/// to the corresponding C++ operators.
/// This means that you can use `&ptr + value` to access the object's `operator+`.
///
/// `MutRef` implements `Deref` and `DerefMut`, allowing to call the object's methods
/// directly. In addition, methods of the object's first base class are also directly available
/// thanks to nested `Deref` implementations.
///
/// If the object provides an iterator interface through `begin()` and `end()` functions,
/// `MutRef` will implement `IntoIterator`, so you can iterate on it directly.
///
/// ### Safety
///
/// It's not possible to automatically track the ownership of objects possibly managed by C++
/// libraries. The user must ensure that the object is alive while `MutRef` exists. Note that
/// with `MutRef`, it's possible to call unsafe C++ code without using any more unsafe Rust code,
/// for example, by using operator traits, so care should be taken when exposing
/// `MutRef` in a safe interface.
pub struct MutRef<T>(ptr::NonNull<T>);

/// Creates another pointer to the same object.
impl<T> Clone for MutRef<T> {
    fn clone(&self) -> Self {
        MutRef(self.0)
    }
}

/// Creates another pointer to the same object.
impl<T> Copy for MutRef<T> {}

impl<T> fmt::Debug for MutRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutRef({:?})", self.0)
    }
}

impl<T> MutRef<T> {
    /// Creates a `MutRef` from a `MutPtr`. Returns `None` if `ptr` is null.
    ///
    /// ### Safety
    ///
    /// `ptr` must be valid. See type level documentation.
    pub unsafe fn new(ptr: MutPtr<T>) -> Option<Self> {
        Self::from_raw(ptr.as_mut_raw_ptr())
    }

    /// Creates a `MutRef` from a raw pointer. Returns `None` if `ptr` is null.
    ///
    /// ### Safety
    ///
    /// `ptr` must be valid. See type level documentation.
    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        ptr::NonNull::new(ptr).map(MutRef)
    }

    /// Creates a `MutRef` from a raw reference.
    ///
    /// ### Safety
    ///
    /// `value` must be alive as long as `MutRef` or pointers derived from it are used.
    /// See type level documentation.
    pub unsafe fn from_raw_ref(value: &mut T) -> Self {
        MutRef(value.into())
    }

    /// Creates a `MutRef` from a non-null pointer.
    ///
    /// ### Safety
    ///
    /// `ptr` must be valid. See type level documentation.
    pub unsafe fn from_raw_non_null(ptr: ptr::NonNull<T>) -> Self {
        MutRef(ptr)
    }

    /// Converts `self` to a `Ptr`.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. See type level documentation.
    pub unsafe fn as_ptr(self) -> Ptr<T> {
        Ptr::from_raw(self.as_raw_ptr())
    }

    /// Converts `self` to a `MutPtr`.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. See type level documentation.
    pub unsafe fn as_mut_ptr(self) -> MutPtr<T> {
        MutPtr::from_raw(self.as_mut_raw_ptr())
    }

    /// Returns constant raw pointer to the value.
    pub fn as_raw_ptr(self) -> *const T {
        self.0.as_ptr()
    }

    /// Returns mutable raw pointer to the value.
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

    /// Converts the pointer to the base class type `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn static_upcast_mut<U>(self) -> MutRef<U>
    where
        T: StaticUpcast<U>,
    {
        StaticUpcast::static_upcast_mut(self.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticUpcast returned null on Ref input")
    }

    /// Converts the pointer to the derived class type `U`.
    ///
    /// It's recommended to use `dynamic_cast` instead because it performs a checked conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid and it's type is `U` or inherits from `U`.
    pub unsafe fn static_downcast_mut<U>(self) -> MutRef<U>
    where
        T: StaticDowncast<U>,
    {
        StaticDowncast::static_downcast_mut(self.as_mut_ptr())
            .as_mut_ref()
            .expect("StaticDowncast returned null on Ref input")
    }

    /// Converts the pointer to the derived class type `U`. Returns `None` if the object's type
    /// is not `U` and doesn't inherit `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid.
    pub unsafe fn dynamic_cast_mut<U>(self) -> Option<MutRef<U>>
    where
        T: DynamicCast<U>,
    {
        DynamicCast::dynamic_cast_mut(self.as_mut_ptr()).as_mut_ref()
    }

    /// Returns a C++ const iterator object pointing to the beginning of the collection.
    ///
    /// It's recommended to iterate directly on a `MutRef<T>` when possible, using automatic
    /// `IntoIterator` implementation.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code.
    pub unsafe fn begin(self) -> <&'static T as Begin>::Output
    where
        &'static T: Begin,
    {
        (*self.as_raw_ptr()).begin()
    }

    /// Returns a C++ mutable iterator object pointing to the beginning of the collection.
    ///
    /// It's recommended to iterate directly on a `MutRef<T>` when possible, using automatic
    /// `IntoIterator` implementation.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code.
    pub unsafe fn begin_mut(self) -> <&'static mut T as BeginMut>::Output
    where
        &'static mut T: BeginMut,
    {
        (*self.as_mut_raw_ptr()).begin_mut()
    }

    /// Returns a C++ const iterator object pointing to the end of the collection.
    ///
    /// It's recommended to iterate directly on a `MutRef<T>` when possible, using automatic
    /// `IntoIterator` implementation.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code.
    pub unsafe fn end(self) -> <&'static T as End>::Output
    where
        &'static T: End,
    {
        (*self.as_raw_ptr()).end()
    }

    /// Returns a C++ mutable iterator object pointing to the end of the collection.
    ///
    /// It's recommended to iterate directly on a `MutRef<T>` when possible, using automatic
    /// `IntoIterator` implementation.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code.
    pub unsafe fn end_mut(self) -> <&'static mut T as EndMut>::Output
    where
        &'static mut T: EndMut,
    {
        (*self.as_mut_raw_ptr()).end_mut()
    }

    /// Returns a slice corresponding to the object. This function is available when `begin()` and
    /// `end()` functions of the object return pointers.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code. It's not recommended to store the slice
    /// because it may be modified by the C++ library, which would violate Rust's aliasing rules.
    pub unsafe fn as_slice<'a, T1>(self) -> &'a [T1]
    where
        T: 'static,
        &'static T: Begin<Output = Ptr<T1>> + End<Output = Ptr<T1>>,
    {
        let begin = self.begin().as_raw_ptr();
        let end = self.end().as_raw_ptr();
        let count = (end as usize).saturating_sub(begin as usize) / mem::size_of::<T1>();
        slice::from_raw_parts(begin, count)
    }

    /// Returns a mutable slice corresponding to the object.
    /// This function is available when `begin()` and
    /// `end()` functions of the object return pointers.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code. It's not recommended to store the slice
    /// because it may be modified by the C++ library, which would violate Rust's aliasing rules.
    pub unsafe fn as_mut_slice<'a, T1>(self) -> &'a mut [T1]
    where
        T: 'static,
        &'static mut T: BeginMut<Output = MutPtr<T1>> + EndMut<Output = MutPtr<T1>>,
    {
        let begin = self.begin_mut().as_mut_raw_ptr();
        let end = self.end_mut().as_mut_raw_ptr();
        let count = (end as usize).saturating_sub(begin as usize) / mem::size_of::<T1>();
        slice::from_raw_parts_mut(begin, count)
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
impl<T> Deref for MutRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
impl<T> DerefMut for MutRef<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

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
/// `Ref` implements `Deref` and `DerefMut`, allowing to call the object's methods
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

    /// Creates a `Ref` from a raw reference.
    ///
    /// ### Safety
    ///
    /// `value` must be alive as long as `Ref` or pointers derived from it are used.
    /// See type level documentation.
    pub unsafe fn from_raw_ref(value: &T) -> Self {
        Ref(ptr::NonNull::new(value as *const T as *mut T).unwrap())
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

    /// Returns constant raw pointer to the value.
    pub fn as_raw_ptr(self) -> *const T {
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

    /// Returns a C++ const iterator object pointing to the beginning of the collection.
    ///
    /// It's recommended to iterate directly on a `Ref<T>` when possible, using automatic
    /// `IntoIterator` implementation.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code.
    pub unsafe fn begin(self) -> <&'static T as Begin>::Output
    where
        &'static T: Begin,
    {
        (*self.as_raw_ptr()).begin()
    }

    /// Returns a C++ const iterator object pointing to the end of the collection.
    ///
    /// It's recommended to iterate directly on a `Ref<T>` when possible, using automatic
    /// `IntoIterator` implementation.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code.
    pub unsafe fn end(self) -> <&'static T as End>::Output
    where
        &'static T: End,
    {
        (*self.as_raw_ptr()).end()
    }

    /// Returns a slice corresponding to the object. This function is available when `begin()` and
    /// `end()` functions of the object return pointers.
    ///
    /// ### Safety
    ///
    /// `self` must be valid. It's not possible to make any guarantees about safety, since
    /// this function calls arbitrary C++ library code. It's not recommended to store the slice
    /// because it may be modified by the C++ library, which would violate Rust's aliasing rules.
    pub unsafe fn as_slice<'a, T1>(self) -> &'a [T1]
    where
        T: 'static,
        &'static T: Begin<Output = Ptr<T1>> + End<Output = Ptr<T1>>,
    {
        let begin = self.begin().as_raw_ptr();
        let end = self.end().as_raw_ptr();
        let count = (end as usize).saturating_sub(begin as usize) / mem::size_of::<T1>();
        slice::from_raw_parts(begin, count)
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}
