//! Various C++-related types and functions needed for the `cpp_to_rust` project.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::ptr;

#[cfg(test)]
mod tests {
    use crate::{CppBox, CppDeletable, Ptr};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Struct1 {
        value: Rc<RefCell<i32>>,
    }

    unsafe extern "C" fn struct1_delete(this_ptr: *mut Struct1) {
        (*this_ptr).value.borrow_mut().clone_from(&42);
    }

    impl CppDeletable for Struct1 {
        unsafe fn delete(&mut self) {
            struct1_delete(self);
        }
    }

    #[test]
    fn test_drop_calls_deleter() {
        let value1 = Rc::new(RefCell::new(10));
        let mut object1 = Struct1 {
            value: value1.clone(),
        };
        assert!(*value1.borrow() == 10);
        unsafe {
            // TODO: remove all "as *mut _" because it's automatic
            CppBox::new(Ptr::new(&mut object1));
        }
        assert!(*value1.borrow() == 42);
    }
}

/// Indicates that the type can be put into a CppBox.
///
/// Example of implementation:
/// ```
/// use cpp_utils::{CppDeletable};
///
/// struct Struct1;
///
/// unsafe extern "C" fn struct1_delete(this_ptr: *mut Struct1) {
///     unimplemented!()
/// }
///
/// impl CppDeletable for Struct1 {
///     unsafe fn delete(&mut self) {
///         struct1_delete(self)
///     }
/// }
/// ```
pub trait CppDeletable: Sized {
    /// Calls C++ `delete x` on `self`.
    unsafe fn delete(&mut self);
}

/// A C++ pointer wrapper to manage deletion of objects.
///
/// Objects of CppBox should be created by calling into_box() for
/// types that implement CppDeletable trait. The object will
/// be deleted when corresponding CppBox is deleted.
pub struct CppBox<T: CppDeletable>(*mut T);

impl<T: CppDeletable> CppBox<T> {
    /// Returns constant raw pointer to the value in the box.
    pub unsafe fn as_ptr(&self) -> ConstPtr<T> {
        ConstPtr::new(self.0)
    }

    /// Returns mutable raw pointer to the value in the box.
    pub unsafe fn as_mut_ptr(&mut self) -> Ptr<T> {
        Ptr::new(self.0)
    }
    /// Returns the pointer that was used to create the object and destroys the box.
    /// The caller of the function becomes the owner of the object and should
    /// ensure that the object will be deleted at some point.
    pub fn into_raw(mut self) -> *mut T {
        let ptr = self.0;
        self.0 = ptr::null_mut();
        ptr
    }

    #[allow(clippy::should_implement_trait)]
    pub unsafe fn as_ref(&self) -> &T {
        self.0.as_ref().unwrap()
    }

    #[allow(clippy::should_implement_trait)]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        self.0.as_mut().unwrap()
    }

    /// Returns true if the pointer is null.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl<T: CppDeletable> CppBox<T> {
    /// Encapsulates the object into a CppBox.
    ///
    /// You should use this function only for
    /// pointers that were created on C++ side and passed through
    /// a FFI boundary to Rust. An object created with C++ `new`
    /// must be deleted using C++ `delete`, which is executed by `CppBox`.
    ///
    /// Do not use this function for objects created in memory managed by Rust.
    /// Any wrapper constructor or function that returns an owned object
    /// is supposed to be deleted using Rust's ownage system and Drop trait.
    ///
    /// Do not use this function for objects that would be deleted by other means.
    /// If another C++ object is the owner of the passed object,
    /// it will attempt to delete it. If `CppBox` containing the object still exists,
    /// it would result in a double deletion, which should never happen.
    ///
    /// Use `CppBox::into_raw` to unwrap the pointer before passing it to
    /// a function that takes ownership of the object.
    ///
    /// It's permitted to put a null pointer into a `CppBox`. Deleter function
    /// will not be called for a null pointer. However, attempting to dereference
    /// a null pointer in a `CppBox`
    /// using `as_ref`, `as_mut`, `deref` or `deref_mut` will result in a panic.
    pub unsafe fn new(ptr: Ptr<T>) -> Self {
        CppBox(ptr.0)
    }

    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        CppBox(ptr)
    }
}

impl<T: CppDeletable> Deref for CppBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.as_ref().unwrap() }
    }
}

impl<T: CppDeletable> DerefMut for CppBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut().unwrap() }
    }
}

impl<T: CppDeletable> Drop for CppBox<T> {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                T::delete(&mut *self.0);
            }
        }
    }
}

impl<T: CppDeletable> Default for CppBox<T> {
    fn default() -> CppBox<T> {
        CppBox(ptr::null_mut())
    }
}

/// Provides access to C++ `static_cast` conversion from derived class to base class.
///
/// This trait is automatically implemented by `cpp_to_rust`.
/// If `T1` class is derived (in C++) from `T2` class,
/// `StaticCast<T2>` is implemented for `T1`.
///
/// `StaticCast` allows to convert a reference to a class into
/// a reference to a base class.
///
/// `static_cast` and `static_cast_mut` free functions can be used
/// to convert pointer types.
///
/// Note that Rust functions associated with this trait have runtime overhead.
/// In C++, `static_cast` is usually a no-op if there is no multiple inheritance,
/// and multiple inheritance requires pointer adjustment. However, Rust compiler
/// and `cpp_to_rust` do not have any information about these implementation details,
/// so all calls of `static_cast` are wrapper in FFI functions.
/// Still, `static_cast` is faster than casts with runtime checks on C++ side
/// because runtime overhead of Rust wrapper functions is the same for all cast types.
pub trait StaticUpcast<T> {
    /// Convert type of a const reference.
    unsafe fn static_upcast(&self) -> ConstPtr<T>;
    /// Convert type of a mutable reference.
    unsafe fn static_upcast_mut(&mut self) -> Ptr<T>;
}

/// Converts type of a const pointer using `StaticCast` implementation of the type.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_upcast<R, T: StaticUpcast<R>>(value: &T) -> ConstPtr<R> {
    value.static_upcast()
}

/// Converts type of a mutable pointer using `StaticCast` implementation of the type.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_upcast_mut<R, T: StaticUpcast<R>>(value: &mut T) -> Ptr<R> {
    value.static_upcast_mut()
}

/// Provides access to C++ `static_cast` conversion from base class to derived class.
///
/// This trait is automatically implemented by `cpp_to_rust`.
/// If `T1` class is derived (in C++) from `T2` class,
/// `UnsafeStaticCast<T1>` is implemented for `T2`.
///
/// `UnsafeStaticCast` allows to convert a reference to a class into
/// a reference to a derived class without runtime check of the type.
/// Casting from base class type to a derived class type which is
/// not the actual type of the object will result in an invalid reference.
///
/// `unsafe_static_cast` and `unsafe_static_cast_mut` free functions can be used
/// to convert pointer types.
///
/// Note that Rust functions associated with this trait have runtime overhead.
/// In C++, `static_cast` is usually a no-op if there is no multiple inheritance,
/// and multiple inheritance requires pointer adjustment. However, Rust compiler
/// and `cpp_to_rust` do not have any information about these implementation details,
/// so all calls of `static_cast` are wrapper in FFI functions.
/// Still, `static_cast` is faster than casts with runtime checks on C++ side
/// because runtime overhead of Rust wrapper functions is the same for all cast types.
pub trait StaticDowncast<T> {
    /// Convert type of a const reference.
    unsafe fn static_downcast(&self) -> ConstPtr<T>;
    /// Convert type of a mutable reference.
    unsafe fn static_downcast_mut(&mut self) -> Ptr<T>;
}

/// Converts type of a const pointer using `UnsafeStaticCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `R` class
/// or a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_downcast<R, T: StaticDowncast<R>>(value: &T) -> ConstPtr<R> {
    value.static_downcast()
}

/// Converts type of a mutable pointer using `UnsafeStaticCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `R` class
/// or a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_downcast_mut<R, T: StaticDowncast<R>>(value: &mut T) -> Ptr<R> {
    value.static_downcast_mut()
}

/// Provides access to C++ `dynamic_cast` conversion.
///
/// This trait is automatically implemented by `cpp_to_rust`.
/// If `T1` class is derived (in C++) from `T2` class,
/// `DynamicCast<T1>` is implemented for `T2`.
/// Use `StaticCast` to convert from `T1` to `T2`.
///
/// `DynamicCast` allows to convert a reference to a class into
/// a reference to a derived class with a runtime check of the type.
/// Conversion returns `None` if the object is actually not an instance of
/// the target type.
///
/// `dynamic_cast` and `dynamic_cast_mut` free functions can be used
/// to convert pointer types.
pub trait DynamicCast<T> {
    /// Convert type of a const reference.
    /// Returns `None` if `self` is not an instance of `T`.
    unsafe fn dynamic_cast(&self) -> Option<ConstPtr<T>>;
    /// Convert type of a mutable reference.
    /// Returns `None` if `self` is not an instance of `T`.
    unsafe fn dynamic_cast_mut(&mut self) -> Option<Ptr<T>>;
}

/// Converts type of a const pointer using `DynamicCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `T` class
/// or a class derived from `T`.
/// Returns null pointer if `ptr` does not point to an instance of `R` or an instance of
/// a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.

pub unsafe fn dynamic_cast<R, T: DynamicCast<R>>(value: &T) -> Option<ConstPtr<R>> {
    value.dynamic_cast()
}

/// Converts type of a mutable pointer using `DynamicCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `T` class
/// or a class derived from `T`.
/// Returns null pointer if `ptr` does not point to an instance of `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn dynamic_cast_mut<R, T: DynamicCast<R>>(value: &mut T) -> Option<Ptr<R>> {
    value.dynamic_cast_mut()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ptr<T>(*mut T);

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Ptr(self.0)
    }
}

impl<T> Copy for Ptr<T> {}

impl<T> Ptr<T> {
    pub unsafe fn new(ptr: *mut T) -> Self {
        Ptr(ptr)
    }

    pub unsafe fn new_option(ptr: *mut T) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Ptr(ptr))
        }
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_ptr(self) -> *const T {
        self.0
    }

    /// Returns mutable raw pointer to the value in the box.
    pub fn as_mut_ptr(self) -> *mut T {
        self.0
    }

    /// Returns true if the pointer is null.
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.0) }
    }
}

impl<T> DerefMut for Ptr<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut (*self.0) }
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
    pub unsafe fn new(ptr: *const T) -> Self {
        ConstPtr(ptr)
    }

    pub unsafe fn new_option(ptr: *const T) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(ConstPtr(ptr))
        }
    }

    /// Returns constant raw pointer to the value in the box.
    pub fn as_ptr(self) -> *const T {
        self.0
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

#[test]
fn ptr_deref() {
    let mut i = 42;
    unsafe {
        let ptr: Ptr<i32> = Ptr::new(&mut i as *mut i32);
        assert_eq!(*ptr, 42);
    }
}
