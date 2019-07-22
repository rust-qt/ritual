//! Various C++-related types and functions needed for the `ritual` project.
//!
//! See [README](https://github.com/rust-qt/ritual) of the repository root for more information.
//!
//! Pointer wrapper types:
//!
//! - `CppBox`: owned, non-null
//! - `Ptr`: possibly owned, possibly null
//! - `Ref`: not owned, non-null

pub use crate::casts::{DynamicCast, StaticDowncast, StaticUpcast};
pub use crate::convert::{CastFrom, CastInto};
pub use crate::cpp_box::{CppBox, CppDeletable};
pub use crate::ptr::{MutPtr, Ptr};
pub use crate::ref_::{MutRef, Ref};

mod casts;
pub mod cmp;
mod convert;
mod cpp_box;
mod ops;
mod ptr;
mod ref_;
