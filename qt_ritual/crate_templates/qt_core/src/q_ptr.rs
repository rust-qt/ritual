use crate::{QBox, QObject, QPointerOfQObject};
use cpp_core::{
    CastFrom, CastInto, CppBox, CppDeletable, DynamicCast, MutPtr, MutRef, Ptr, Ref,
    StaticDowncast, StaticUpcast,
};
use std::fmt;
use std::ops::{Deref, DerefMut};

/// A smart pointer that automatically resets when the object is deleted.
///
/// `QPtr` exposes functionality provided by the `QPointer<T>` C++ class.
/// `QPtr` can only contain a pointer to a `QObject`-based object. When that object is
/// deleted, `QPtr` automatically becomes a null pointer.
///
/// Note that dereferencing a null `QPtr` will panic, so if it's known that the object may
/// already have been deleted, you should use `is_null()`, `as_mut_ref()`,
/// or a similar method to check
/// if the object is still alive before calling its methods.
///
/// `QPtr` is not an owning pointer, similar to `cpp_core::MutPtr`. If you actually own the object,
/// you should convert it to `QBox` (it will delete the object when dropped if it has no parent)
/// or `CppBox` (it will always delete the object when dropped). `QPtr` provides `into_qbox` and
/// `to_box` helpers for that.
///
/// # Safety
///
/// While `QPtr` is much safer than `cpp_core::MutPtr` and prevents use-after-free in common cases,
/// it is unsafe to use in Rust terms. `QPtr::new` must receive a valid pointer or a null pointer,
/// otherwise the behavior is undefined. You should not store pointers of other types
/// (e.g. `MutPtr`, `MutRef`, or raw pointers) produced by `QPtr` because, unlike `QPtr`, these
/// pointers will not become null pointers when the object is deleted.
///
/// It's still possible to cause use-after-free by calling a method through `QPtr`.
/// Even in a single threaded program, the accessed object can be deleted by a nested call
/// while one of its methods is still running. In multithreaded context, the object can be deleted
/// in another thread between the null check and the method call, also resulting in undefined
/// behavior.
pub struct QPtr<T: StaticUpcast<QObject>> {
    q_pointer: Option<CppBox<QPointerOfQObject>>,
    target: MutPtr<T>,
}

impl<T: StaticUpcast<QObject>> QPtr<T> {
    /// Creates a `QPtr` from a `MutPtr`.
    ///
    /// ### Safety
    ///
    /// `target` must be either a valid pointer to an object or a null pointer.
    /// See type level documentation.
    pub unsafe fn new(target: impl CastInto<MutPtr<T>>) -> Self {
        let target = target.cast_into();
        QPtr {
            q_pointer: if target.is_null() {
                None
            } else {
                Some(QPointerOfQObject::new_1a(target))
            },
            target,
        }
    }

    /// Creates a `QPtr` from a raw pointer.
    ///
    /// ### Safety
    ///
    /// `target` must be either a valid pointer to an object or a null pointer.
    /// See type level documentation.
    pub unsafe fn from_raw(target: *mut T) -> Self {
        Self::new(MutPtr::from_raw(target))
    }

    /// Creates a null pointer.
    ///
    /// Note that you can also use `NullPtr` to specify a null pointer to a function accepting
    /// `impl CastInto<MutPtr<_>>`. Unlike `MutPtr`, `NullPtr` is not a generic type, so it will
    /// not cause type inference issues.
    ///
    /// Note that accessing the content of a null `QPtr` through `Deref` or `DerefMut` will result
    /// in a panic.
    ///
    /// ### Safety
    ///
    /// Null pointers must not be dereferenced. See type level documentation.
    pub unsafe fn null() -> Self {
        Self::new(MutPtr::<T>::null())
    }

    /// Returns true if the pointer is null.
    pub unsafe fn is_null(&self) -> bool {
        self.q_pointer.as_ref().map_or(true, |p| p.is_null())
    }

    /// Returns the content as a `MutPtr`.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_mut_ptr(&mut self) -> MutPtr<T> {
        if self.is_null() {
            MutPtr::null()
        } else {
            self.target
        }
    }

    /// Returns the content as a const `Ptr`.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_ptr(&self) -> Ptr<T> {
        if self.is_null() {
            Ptr::null()
        } else {
            self.target.as_ptr()
        }
    }

    /// Returns the content as a raw const pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_raw_ptr(&self) -> *const T {
        self.as_ptr().as_raw_ptr()
    }

    /// Returns the content as a raw mutable pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_mut_raw_ptr(&mut self) -> *mut T {
        self.as_mut_ptr().as_mut_raw_ptr()
    }

    /// Returns the content as a const `Ref`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_ref(&self) -> Option<Ref<T>> {
        self.as_ptr().as_ref()
    }

    /// Returns the content as a `MutRef`. Returns `None` if `self` is a null pointer.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn as_mut_ref(&mut self) -> Option<MutRef<T>> {
        self.as_mut_ptr().as_mut_ref()
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
        QPtr::<U>::new(self.as_mut_ptr().static_upcast_mut::<U>())
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
        QPtr::<U>::new(self.as_mut_ptr().static_downcast_mut())
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
        QPtr::<U>::new(self.as_mut_ptr().dynamic_cast_mut())
    }

    /// Converts this pointer to a `CppBox`. Returns `None` if `self`
    /// is a null pointer.
    ///
    /// Use this function to take ownership of the object. This is
    /// the same as `CppBox::new`. `CppBox` will delete the object when dropped.
    ///
    /// You can also use `into_qbox` to convert the pointer to a `QBox`.
    /// Unlike `CppBox`, `QBox` will only delete the object if it has no parent.
    ///
    /// ### Safety
    ///
    /// `CppBox` will attempt to delete the object on drop. If something else also tries to
    /// delete this object before or after that, the behavior is undefined.
    /// See type level documentation.
    pub unsafe fn to_box(&mut self) -> Option<CppBox<T>>
    where
        T: CppDeletable,
    {
        self.as_mut_ptr().to_box()
    }

    /// Converts this pointer to a `QBox`.
    ///
    /// Use this function to take ownership of the object. This is
    /// the same as `QBox::from_qptr`.
    ///
    /// ### Safety
    ///
    /// See type level documentation.
    pub unsafe fn into_qbox(self) -> QBox<T>
    where
        T: CppDeletable,
    {
        QBox::from_qptr(self)
    }
}

/// Creates another pointer to the same object.
impl<T: StaticUpcast<QObject>> Clone for QPtr<T> {
    fn clone(&self) -> Self {
        unsafe { QPtr::<T>::new(MutPtr::from_raw(self.as_raw_ptr() as *mut T)) }
    }
}

impl<T: StaticUpcast<QObject>> fmt::Debug for QPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "QPtr({:?})", self.target.as_mut_raw_ptr())
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
///
/// Panics if the pointer is null.
impl<T: StaticUpcast<QObject>> Deref for QPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            let ptr = self.as_raw_ptr();
            if ptr.is_null() {
                panic!("attempted to deref a null QPtr<T>");
            }
            &*ptr
        }
    }
}

/// Allows to call member functions of `T` and its base classes directly on the pointer.
///
/// Panics if the pointer is null.
impl<T: StaticUpcast<QObject>> DerefMut for QPtr<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            let ptr = self.as_mut_raw_ptr();
            if ptr.is_null() {
                panic!("attempted to deref a null QPtr<T>");
            }
            &mut *ptr
        }
    }
}

impl<'a, T, U> CastFrom<&'a QPtr<U>> for Ptr<T>
where
    U: StaticUpcast<T> + StaticUpcast<QObject>,
{
    unsafe fn cast_from(value: &'a QPtr<U>) -> Self {
        CastFrom::cast_from(value.as_ptr())
    }
}

impl<'a, T, U> CastFrom<&'a mut QPtr<U>> for MutPtr<T>
where
    U: StaticUpcast<T> + StaticUpcast<QObject>,
{
    unsafe fn cast_from(value: &'a mut QPtr<U>) -> Self {
        CastFrom::cast_from(value.as_mut_ptr())
    }
}

impl<T, U> CastFrom<QPtr<U>> for Ptr<T>
where
    U: StaticUpcast<T> + StaticUpcast<QObject>,
{
    unsafe fn cast_from(value: QPtr<U>) -> Self {
        CastFrom::cast_from(value.as_ptr())
    }
}

impl<T, U> CastFrom<QPtr<U>> for MutPtr<T>
where
    U: StaticUpcast<T> + StaticUpcast<QObject>,
{
    unsafe fn cast_from(mut value: QPtr<U>) -> Self {
        CastFrom::cast_from(value.as_mut_ptr())
    }
}
