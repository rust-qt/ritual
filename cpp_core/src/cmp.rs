//! Comparison operator traits
//!
//! C++'s comparison operators have different semantics from Rust's `PartialOrd` and `Ord` traits.
//! If all the operators (`Lt`, `Le`, `Gt`, `Ge`) are implemented for a type, the pointer types
//! (`CppBox`, `Ptr`, `Ref`) automatically implement `PartialOrd`.

/// Represents C++'s `operator<`.
pub trait Lt<Rhs: ?Sized = Self> {
    /// This method tests less than (for `self` and `other`) and is used by the `<` operator.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` and `other` contain valid pointers. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn lt(&self, other: &Rhs) -> bool;
}

/// Represents C++'s `operator<=`.
pub trait Le<Rhs: ?Sized = Self> {
    /// This method tests less than or equal to (for `self` and `other`) and is used by the `<=`
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` and `other` contain valid pointers. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn le(&self, other: &Rhs) -> bool;
}

/// Represents C++'s `operator>`.
pub trait Gt<Rhs: ?Sized = Self> {
    /// This method tests greater than (for `self` and `other`) and is used by the `>` operator.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` and `other` contain valid pointers. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn gt(&self, other: &Rhs) -> bool;
}

/// Represents C++'s `operator>=`.
pub trait Ge<Rhs: ?Sized = Self> {
    /// This method tests greater than or equal to (for `self` and `other`) and is used by the `>=`
    /// operator.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` and `other` contain valid pointers. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn ge(&self, other: &Rhs) -> bool;
}
