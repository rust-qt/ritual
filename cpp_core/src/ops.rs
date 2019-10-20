//! Operator traits not present in Rust's `std`
//!
//! See also `cpp_core::cmp` for comparison operator traits.

// TODO: `&mut self` for increment and decrement?

/// Represents C++'s prefix increment (`++a`).
pub trait Increment {
    /// Output type.
    type Output;

    /// Increment `self`.
    unsafe fn inc(&mut self) -> Self::Output;
}

/// Represents C++'s prefix decrement (`--a`).
pub trait Decrement {
    /// Output type.
    type Output;

    /// Decrement `self`.
    unsafe fn dec(&mut self) -> Self::Output;
}

/// Represents C++'s indirection operator (`*a`).
pub trait Indirection {
    /// Output type.
    type Output;

    /// Returns the object `self` is pointing to.
    unsafe fn indirection(&self) -> Self::Output;
}

/// Represents C++'s `begin() const` function.
pub trait Begin {
    /// Output type.
    type Output;

    /// Returns a C++ const iterator object pointing to the beginning of the collection.
    unsafe fn begin(&self) -> Self::Output;
}

/// Represents C++'s `begin()` function.
pub trait BeginMut {
    /// Output type.
    type Output;

    /// Returns a C++ mutable iterator object pointing to the beginning of the collection.
    unsafe fn begin_mut(&mut self) -> Self::Output;
}

/// Represents C++'s `end() const` function.
pub trait End {
    /// Output type.
    type Output;

    /// Returns a C++ const iterator object pointing to the end of the collection.
    unsafe fn end(&self) -> Self::Output;
}

/// Represents C++'s `end()` function.
pub trait EndMut {
    /// Output type.
    type Output;

    /// Returns a C++ mutable iterator object pointing to the end of the collection.
    unsafe fn end_mut(&mut self) -> Self::Output;
}
