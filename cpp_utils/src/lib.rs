//! Various C++-related types and functions needed for the `ritual` project.
//!
//! Pointer wrapper types:
//!
//! - `CppBox`: owned, non-null
//! - `Ptr`: possibly owned, possibly null
//! - `Ref`: not owned, non-null

pub use crate::casts::{
    dynamic_cast, dynamic_cast_mut, static_downcast, static_downcast_mut, static_upcast,
    static_upcast_mut, DynamicCast, StaticDowncast, StaticUpcast,
};
pub use crate::cpp_box::{CppBox, CppDeletable};
pub use crate::ptr::{ConstPtr, Ptr};
pub use crate::ref_::{ConstRef, Ref};

mod casts;
mod cpp_box;
mod ptr;
mod ref_;
