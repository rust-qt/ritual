use crate::{QObject, QPtr};
use cpp_core::{
    CastFrom, CastInto, CppBox, CppDeletable, DynamicCast, MutPtr, MutRef, Ptr, Ref,
    StaticDowncast, StaticUpcast,
};
use std::ops::{Deref, DerefMut};
use std::{fmt, mem};

/// An owning pointer for `QObject`-based objects.
///
/// `QBox` will delete its object on drop if it has no parent. If the object has a parent,
/// it's assumed that the parent is responsible for deleting the object, as per Qt ownership system.
/// Additionally, `QBox` will be automatically set to null when the object is deleted, similar
/// to `QPtr` (or `QPointer<T>` in C++). `QBox` will not attempt to delete null pointers.
///
/// Note that dereferencing a null `QBox` will panic, so if it's known that the object may
/// already have been deleted, you should use `is_null()`, `as_mut_ref()`,
/// or a similar method to check
/// if the object is still alive before calling its methods.
///
/// Unlike `CppBox` (which is non-nullable), `QBox` is permitted to contain a null pointer because
/// even if a non-null pointer is provided when constructing `QBox`, it will become null
/// automatically if the object is deleted.
///
/// To prevent the object from being deleted, convert `QBox` to another type of pointer using
/// `into_qptr()` or `into_ptr()`. Alternatively, setting a parent for the object will prevent
/// `QBox` from deleting it.
///
/// To make sure the object is deleted regardless of its parent, convert `QBox` to `CppBox` using
/// `into_box()`.
///
/// # Safety
///
/// `QBox` has the same safety issues as `QPtr`. See `QPtr` documentation.
pub struct QBox<T: StaticUpcast<QObject> + CppDeletable>(QPtr<T>);

impl<T: StaticUpcast<QObject> + CppDeletable> QBox<T> {
    /// Creates a `QBox` from a `QPtr`.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn from_qptr(target: QPtr<T>) -> Self {
        QBox(target)
    }

    /// Creates a `QBox` from a `MutPtr`.
    ///
    /// ### Safety
    ///
    /// `target` must be either a valid pointer to an object or a null pointer.
    /// See type level documentation.
    pub unsafe fn new(target: impl CastInto<MutPtr<T>>) -> Self {
        QBox::from_qptr(QPtr::new(target))
    }

    /// Creates a `QBox` from a raw pointer.
    ///
    /// ### Safety
    ///
    /// `target` must be either a valid pointer to an object or a null pointer.
    /// See type level documentation.
    pub unsafe fn from_raw(target: *mut T) -> Self {
        QBox::from_qptr(QPtr::from_raw(target))
    }

    /// Creates a null pointer.
    ///
    /// Note that you can also use `NullPtr` to specify a null pointer to a function accepting
    /// `impl CastInto<MutPtr<_>>`. Unlike `MutPtr`, `NullPtr` is not a generic type, so it will
    /// not cause type inference issues.
    ///
    /// Note that accessing the content of a null `QBox` through `Deref` or `DerefMut` will result
    /// in a panic.
    ///
    /// ### Safety
    ///
    /// Null pointers must not be dereferenced. See type level documentation.
    pub unsafe fn null() -> Self {
        QBox::from_qptr(QPtr::<T>::null())
    }

    /// Returns true if the pointer is null.
    pub unsafe fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Returns the content as a `MutPtr`.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_mut_ptr(&mut self) -> MutPtr<T> {
        self.0.as_mut_ptr()
    }

    /// Returns the content as a const `Ptr`.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_ptr(&self) -> Ptr<T> {
        self.0.as_ptr()
    }

    /// Returns the content as a raw const pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_raw_ptr(&self) -> *const T {
        self.0.as_raw_ptr()
    }

    /// Returns the content as a raw mutable pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_mut_raw_ptr(&mut self) -> *mut T {
        self.0.as_mut_raw_ptr()
    }

    /// Returns the content as a const `Ref`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_ref(&self) -> Option<Ref<T>> {
        self.0.as_ref()
    }

    /// Returns the content as a `MutRef`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_mut_ref(&mut self) -> Option<MutRef<T>> {
        self.0.as_mut_ref()
    }

    /// Converts the pointer to the base class type `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid or null. See type level documentation.
    pub unsafe fn static_upcast_mut<U>(&mut self) -> QPtr<U>
    where
        T: StaticUpcast<U>,
        U: StaticUpcast<QObject>,
    {
        self.0.static_upcast_mut()
    }

    /// Converts the pointer to the derived class type `U`.
    ///
    /// It's recommended to use `dynamic_cast` instead because it performs a checked conversion.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid and it's type is `U` or inherits from `U`,
    /// of if `self` is a null pointer. See type level documentation.
    pub unsafe fn static_downcast_mut<U>(&mut self) -> QPtr<U>
    where
        T: StaticDowncast<U>,
        U: StaticUpcast<QObject>,
    {
        self.0.static_downcast_mut()
    }

    /// Converts the pointer to the derived class type `U`. Returns `None` if the object's type
    /// is not `U` and doesn't inherit `U`.
    ///
    /// ### Safety
    ///
    /// This operation is safe as long as `self` is valid or null. See type level documentation.
    pub unsafe fn dynamic_cast_mut<U>(&mut self) -> QPtr<U>
    where
        T: DynamicCast<U>,
        U: StaticUpcast<QObject>,
    {
        self.0.dynamic_cast_mut()
    }

    /// Converts this pointer to a `CppBox`. Returns `None` if `self`
    /// is a null pointer.
    ///
    /// Unlike `QBox`, `CppBox` will always delete the object when dropped.
    ///
    /// ### Safety
    ///
    /// `CppBox` will attempt to delete the object on drop. If something else also tries to
    /// delete this object before or after that, the behavior is undefined.
    /// See type level documentation.
    pub unsafe fn into_box(self) -> Option<CppBox<T>> {
        self.into_qptr().to_box()
    }

    pub unsafe fn into_qptr(mut self) -> QPtr<T> {
        mem::replace(&mut self.0, QPtr::null())
    }

    pub unsafe fn into_ptr(self) -> MutPtr<T> {
        self.into_qptr().as_mut_ptr()
    }

    pub unsafe fn into_raw_ptr(self) -> *mut T {
        self.into_qptr().as_mut_raw_ptr()
    }
}

impl<T: StaticUpcast<QObject> + CppDeletable> fmt::Debug for QBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "QBox({:?})", unsafe { self.as_raw_ptr() })
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
///
/// Panics if the pointer is null.
impl<T: StaticUpcast<QObject> + CppDeletable> Deref for QBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            let ptr = self.as_raw_ptr();
            if ptr.is_null() {
                panic!("attempted to deref a null QBox<T>");
            }
            &*ptr
        }
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
///
/// Panics if the pointer is null.
impl<T: StaticUpcast<QObject> + CppDeletable> DerefMut for QBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            let ptr = self.as_mut_raw_ptr();
            if ptr.is_null() {
                panic!("attempted to deref a null QBox<T>");
            }
            &mut *ptr
        }
    }
}

impl<'a, T, U> CastFrom<&'a QBox<U>> for Ptr<T>
where
    U: StaticUpcast<T> + StaticUpcast<QObject> + CppDeletable,
{
    unsafe fn cast_from(value: &'a QBox<U>) -> Self {
        CastFrom::cast_from(value.as_ptr())
    }
}

impl<'a, T, U> CastFrom<&'a mut QBox<U>> for MutPtr<T>
where
    U: StaticUpcast<T> + StaticUpcast<QObject> + CppDeletable,
{
    unsafe fn cast_from(value: &'a mut QBox<U>) -> Self {
        CastFrom::cast_from(value.as_mut_ptr())
    }
}

impl<T: StaticUpcast<QObject> + CppDeletable> Drop for QBox<T> {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.as_mut_ptr();
            if !ptr.is_null() && ptr.static_upcast_mut().parent().is_null() {
                T::delete(&mut *ptr.as_mut_raw_ptr());
            }
        }
    }
}
