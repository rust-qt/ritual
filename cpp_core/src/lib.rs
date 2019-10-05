//! Utilities for interoperability with C++
//!
//! See the project's [README](https://github.com/rust-qt/ritual) for more information.
//!
//! The API is not stable yet. Breaking changes may occur in new minor versions.
//!
//! # Pointers
//!
//! `cpp_core` provides three kinds of pointers:
//!
//! - `CppBox`: owned, non-null (corresponds to C++ objects passed by value)
//! - `Ptr` and `MutPtr`: possibly owned, possibly null (correspond to C++ pointers)
//! - `Ref` and `MutRef`: not owned, non-null (correspond to C++ references)
//!
//! Accessing objects through these pointers is inherently unsafe,
//! as the compiler cannot make any guarantee about the validity of pointers to objects
//! managed by C++ libraries.
//!
//! Unlike Rust references, these pointers can be freely copied,
//! producing multiple mutable pointers to the same object, which is usually necessary
//! to do when working with C++ libraries.
//!
//! Pointer types implement operator traits and delegate them to the corresponding C++ operators.
//! This means that you can use `ptr1 + ptr2` to access the object's `operator+`.
//!
//! Pointer types implement `Deref` and `DerefMut`, allowing to call the object's methods
//! directly. In addition, methods of the object's first base class are also directly available
//! thanks to nested `Deref` implementations.
//!
//! If the object provides an iterator interface through `begin()` and `end()` functions,
//! pointer types will implement `IntoIterator`, so you can iterate on them directly.
//!
//! # Casts
//!
//! The following traits provide access to casting between C++ class types:
//!
//! - `StaticUpcast` safely converts from a derived class to a base class
//! (backed by C++'s `static_cast`).
//! - `DynamicCast` performs a checked conversion from a base class to a derived class
//! (backed by C++'s `dynamic_cast`).
//! - `StaticDowncast` converts from a base class to a derived class without a runtime
//! check (also backed by C++'s `static_cast`).
//!
//! Instead of using these traits directly, it's more convenient to use `static_upcast`,
//! `static_downcast`, `dynamic_cast` helpers on pointer types.
//!
//! The `CastFrom` and `CastInto` traits represent some of the implicit coercions
//! available in C++. For example, if a method accepts `impl CastInto<Ptr<SomeClass>>`,
//! you can pass a `Ptr<SomeClass>`, `MutPtr<SomeClass>`, `&CppBox<SomeClass>`,
//! or even `Ptr<DerivedClass>` (where `DerivedClass` inherits `SomeClass`). You can also
//! pass a null pointer object (`NullPtr`) if you don't have a value
//! (`Ptr::null()` is also an option but it can cause type inference issues).

#![deny(missing_docs)]

pub use crate::casts::{DynamicCast, StaticDowncast, StaticUpcast};
pub use crate::convert::{CastFrom, CastInto};
pub use crate::cpp_box::{CppBox, CppDeletable};
pub use crate::iterator::{cpp_iter, CppIterator};
pub use crate::ptr::{MutPtr, NullPtr, Ptr};
pub use crate::ref_::{MutRef, Ref};
pub use libc::wchar_t;

mod casts;
pub mod cmp;
mod convert;
mod cpp_box;
mod iterator;
pub mod ops;
mod ops_impls;
mod ptr;
mod ref_;

// C++ doesn't guarantee these types to be exactly u16 and u32,
// but they are on all supported platforms.

/// Type for UTF-16 character representation, required to be large enough to represent
/// any UTF-16 code unit (16 bits). Same as C++'s `char16_6` type.
#[allow(non_camel_case_types)]
pub type char16_t = u16;
/// Type for UTF-32 character representation, required to be large enough to represent
/// any UTF-32 code unit (32 bits). Same as C++'s `char32_t` type.
#[allow(non_camel_case_types)]
pub type char32_t = u32;
