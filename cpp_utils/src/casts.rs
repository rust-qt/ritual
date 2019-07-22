use crate::{MutRef, Ref};

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
pub trait StaticUpcast<T> {
    /// Convert type of a const reference.
    unsafe fn static_upcast(&self) -> Ref<T>;
    /// Convert type of a mutable reference.
    unsafe fn static_upcast_mut(&mut self) -> MutRef<T>;
}

/// Converts type of a const pointer using `StaticCast` implementation of the type.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_upcast<R, T: StaticUpcast<R>>(value: &T) -> Ref<R> {
    value.static_upcast()
}

/// Converts type of a mutable pointer using `StaticCast` implementation of the type.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_upcast_mut<R, T: StaticUpcast<R>>(value: &mut T) -> MutRef<R> {
    value.static_upcast_mut()
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
pub trait StaticDowncast<T> {
    /// Convert type of a const reference.
    unsafe fn static_downcast(&self) -> Ref<T>;
    /// Convert type of a mutable reference.
    unsafe fn static_downcast_mut(&mut self) -> MutRef<T>;
}

/// Converts type of a const pointer using `UnsafeStaticCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `R` class
/// or a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_downcast<R, T: StaticDowncast<R>>(value: &T) -> Ref<R> {
    value.static_downcast()
}

/// Converts type of a mutable pointer using `UnsafeStaticCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `R` class
/// or a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn static_downcast_mut<R, T: StaticDowncast<R>>(value: &mut T) -> MutRef<R> {
    value.static_downcast_mut()
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
pub trait DynamicCast<T> {
    /// Convert type of a const reference.
    /// Returns `None` if `self` is not an instance of `T`.
    unsafe fn dynamic_cast(&self) -> Option<Ref<T>>;
    /// Convert type of a mutable reference.
    /// Returns `None` if `self` is not an instance of `T`.
    unsafe fn dynamic_cast_mut(&mut self) -> Option<MutRef<T>>;
}

/// Converts type of a const pointer using `DynamicCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `T` class
/// or a class derived from `T`.
/// Returns null pointer if `ptr` does not point to an instance of `R` or an instance of
/// a class derived from `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.

pub unsafe fn dynamic_cast<R, T: DynamicCast<R>>(value: &T) -> Option<Ref<R>> {
    value.dynamic_cast()
}

/// Converts type of a mutable pointer using `DynamicCast` implementation of the type.
/// `ptr` must be either a null pointer or a valid pointer to an instance of `T` class
/// or a class derived from `T`.
/// Returns null pointer if `ptr` does not point to an instance of `R`.
/// If `ptr` is null, this function does nothing and returns null pointer.
pub unsafe fn dynamic_cast_mut<R, T: DynamicCast<R>>(value: &mut T) -> Option<MutRef<R>> {
    value.dynamic_cast_mut()
}
