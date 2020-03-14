//! Traits for common operations on C++ vectors.

/// Provides access to the underlying memory buffer.
pub trait Data {
    /// Return type of `data()` function.
    type Output;
    /// Returns a pointer to the underlying memory buffer.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn data(&self) -> Self::Output;
}

/// Provides mutable access to the underlying memory buffer.
pub trait DataMut {
    /// Return type of `data_mut()` function.
    type Output;
    /// Returns a pointer to the underlying memory buffer.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn data_mut(&self) -> Self::Output;
}

/// Provides access to the size of the collection.
pub trait Size {
    /// Returns number of the elements in the underlying buffer.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn size(&self) -> usize;
}
