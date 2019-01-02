//! Various C++-related types and functions needed for the `cpp_to_rust` project.

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use {CppBox, CppDeletable, Deleter};

    struct Struct1 {
        value: Rc<RefCell<i32>>,
    }

    unsafe extern "C" fn struct1_delete(this_ptr: *mut Struct1) {
        (*this_ptr).value.borrow_mut().clone_from(&mut 42);
    }

    impl CppDeletable for Struct1 {
        fn deleter() -> Deleter<Self> {
            struct1_delete
        }
    }

    #[test]
    fn test_drop_calls_deleter() {
        let value1 = Rc::new(RefCell::new(10));
        let mut object1 = Struct1 {
            value: value1.clone(),
        };
        assert!(value1.borrow().clone() == 10);
        unsafe {
            CppBox::new(&mut object1 as *mut _);
        }
        assert!(value1.borrow().clone() == 42);
    }
}

/// Deleter function type.
///
/// This is usually a C++ function imported via FFI
/// from a wrapper library. The body of this function
/// should be "delete this_ptr;".
pub type Deleter<T> = unsafe extern "C" fn(this_ptr: *mut T);

/// Indicates that the type can be put into a CppBox.
///
/// Example of implementation:
/// ```
/// impl CppDeletable for Struct1 {
///   fn deleter() -> Deleter<Self> {
///     struct1_delete
///   }
/// }
/// ```
pub trait CppDeletable: Sized {
    /// Returns deleter function for this type.
    fn deleter() -> Deleter<Self>;
}

/// A C++ pointer wrapper to manage deletion of objects.
///
/// Objects of CppBox should be created by calling into_box() for
/// types that implement CppDeletable trait. The object will
/// be deleted when corresponding CppBox is deleted.
pub struct CppBox<T: CppDeletable> {
    ptr: *mut T,
    deleter: Deleter<T>,
}

impl<T: CppDeletable> CppBox<T> {
    /// Returns constant raw pointer to the value in the box.
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }
    /// Returns mutable raw pointer to the value in the box.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }
    /// Returns the pointer that was used to create the object and destroys the box.
    /// The caller of the function becomes the owner of the object and should
    /// ensure that the object will be deleted at some point.
    pub fn into_raw(mut self) -> *mut T {
        let ptr = self.ptr;
        self.ptr = std::ptr::null_mut();
        ptr
    }

    /// Returns true if the pointer is null.
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
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
    pub unsafe fn new(ptr: *mut T) -> CppBox<T> {
        CppBox {
            ptr: ptr,
            deleter: CppDeletable::deleter(),
        }
    }
}

impl<T: CppDeletable> AsRef<T> for CppBox<T> {
    fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T: CppDeletable> AsMut<T> for CppBox<T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<T: CppDeletable> std::ops::Deref for CppBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref().unwrap() }
    }
}

impl<T: CppDeletable> std::ops::DerefMut for CppBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut().unwrap() }
    }
}

impl<T: CppDeletable> Drop for CppBox<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                (self.deleter)(self.ptr);
            }
        }
    }
}

impl<T: CppDeletable> Default for CppBox<T> {
    fn default() -> CppBox<T> {
        CppBox {
            ptr: std::ptr::null_mut(),
            deleter: CppDeletable::deleter(),
        }
    }
}

/// This module contains `NewUninitialized` trait.
/// It's an implementation detail of `cpp_to_rust` and should not be used directly.
pub mod new_uninitialized {

    /// A trait for types that can be created with
    /// uninitialized internal buffer.
    ///
    /// This trait is an implementation detail of `cpp_to_rust` and should not be used directly.
    pub trait NewUninitialized {
        /// Creates new object with uninitialized internal buffer.
        unsafe fn new_uninitialized() -> Self;
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
pub trait StaticCast<T> {
    /// Convert type of a const reference.
    fn static_cast(&self) -> &T;
    /// Convert type of a mutable reference.
    fn static_cast_mut(&mut self) -> &mut T;
}

/// Converts type of a const pointer using `StaticCast` implementation of the type.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub fn static_cast<R, T: StaticCast<R>>(ptr: *const T) -> *const R {
    unsafe { ptr.as_ref() }
        .map(|x| x.static_cast() as *const R)
        .unwrap_or(std::ptr::null())
}

/// Converts type of a mutable pointer using `StaticCast` implementation of the type.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub fn static_cast_mut<R, T: StaticCast<R>>(ptr: *mut T) -> *mut R {
    unsafe { ptr.as_mut() }
        .map(|x| x.static_cast_mut() as *mut R)
        .unwrap_or(std::ptr::null_mut())
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
pub trait UnsafeStaticCast<T> {
    /// Convert type of a const reference.
    unsafe fn static_cast(&self) -> &T;
    /// Convert type of a mutable reference.
    unsafe fn static_cast_mut(&mut self) -> &mut T;
}

/// Converts type of a const pointer using `UnsafeStaticCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `R` class
/// or a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn unsafe_static_cast<R, T: UnsafeStaticCast<R>>(ptr: *const T) -> *const R {
    ptr.as_ref()
        .map(|x| x.static_cast() as *const R)
        .unwrap_or(std::ptr::null())
}

/// Converts type of a mutable pointer using `UnsafeStaticCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `R` class
/// or a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn unsafe_static_cast_mut<R, T: UnsafeStaticCast<R>>(ptr: *mut T) -> *mut R {
    ptr.as_mut()
        .map(|x| x.static_cast_mut() as *mut R)
        .unwrap_or(std::ptr::null_mut())
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
    fn dynamic_cast(&self) -> Option<&T>;
    /// Convert type of a mutable reference.
    /// Returns `None` if `self` is not an instance of `T`.
    fn dynamic_cast_mut(&mut self) -> Option<&mut T>;
}

/// Converts type of a const pointer using `DynamicCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `T` class
/// or a class derived from `T`.
/// Returns null pointer if `ptr` does not point to an instance of `R` or an instance of
/// a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn dynamic_cast<R, T: DynamicCast<R>>(ptr: *const T) -> *const R {
    ptr.as_ref()
        .and_then(|x| x.dynamic_cast())
        .map(|x| x as *const R)
        .unwrap_or(std::ptr::null())
}

/// Converts type of a mutable pointer using `DynamicCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `T` class
/// or a class derived from `T`.
/// Returns null pointer if `ptr` does not point to an instance of `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn dynamic_cast_mut<R, T: DynamicCast<R>>(ptr: *mut T) -> *mut R {
    ptr.as_mut()
        .and_then(|x| x.dynamic_cast_mut())
        .map(|x| x as *mut R)
        .unwrap_or(std::ptr::null_mut())
}
