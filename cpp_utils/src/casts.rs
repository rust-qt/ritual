use crate::{MutPtr, Ptr};

/// Provides access to C++ `static_cast` conversion from derived class to base class.
///
/// This trait is automatically implemented by `ritual`.
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
/// and `ritual` do not have any information about these implementation details,
/// so all calls of `static_cast` are wrapper in FFI functions.
/// Still, `static_cast` is faster than casts with runtime checks on C++ side
/// because runtime overhead of Rust wrapper functions is the same for all cast types.
pub trait StaticUpcast<T>: Sized {
    /// Convert type of a const reference.
    unsafe fn static_upcast(ptr: Ptr<Self>) -> Ptr<T>;
    /// Convert type of a mutable reference.
    unsafe fn static_upcast_mut(ptr: MutPtr<Self>) -> MutPtr<T>;
}

impl<T> StaticUpcast<T> for T {
    unsafe fn static_upcast(ptr: Ptr<T>) -> Ptr<T> {
        ptr
    }

    unsafe fn static_upcast_mut(ptr: MutPtr<T>) -> MutPtr<T> {
        ptr
    }
}

/// Provides access to C++ `static_cast` conversion from base class to derived class.
///
/// This trait is automatically implemented by `ritual`.
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
/// and `ritual` do not have any information about these implementation details,
/// so all calls of `static_cast` are wrapper in FFI functions.
/// Still, `static_cast` is faster than casts with runtime checks on C++ side
/// because runtime overhead of Rust wrapper functions is the same for all cast types.
pub trait StaticDowncast<T>: Sized {
    /// Convert type of a const reference.
    unsafe fn static_downcast(ptr: Ptr<Self>) -> Ptr<T>;
    /// Convert type of a mutable reference.
    unsafe fn static_downcast_mut(ptr: MutPtr<Self>) -> MutPtr<T>;
}

/// Provides access to C++ `dynamic_cast` conversion.
///
/// This trait is automatically implemented by `ritual`.
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
pub trait DynamicCast<T>: Sized {
    /// Convert type of a const reference.
    /// Returns `None` if `self` is not an instance of `T`.
    unsafe fn dynamic_cast(ptr: Ptr<Self>) -> Ptr<T>;
    /// Convert type of a mutable reference.
    /// Returns `None` if `self` is not an instance of `T`.
    unsafe fn dynamic_cast_mut(ptr: MutPtr<Self>) -> MutPtr<T>;
}
