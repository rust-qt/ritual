//! Traits for common operations on C++ vectors.

pub trait Data {
    type Output;
    /// Returns a pointer to the underlying memory buffer.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn data(&self) -> Self::Output;
}

pub trait DataMut {
    type Output;
    /// Returns a pointer to the underlying memory buffer.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn data_mut(&self) -> Self::Output;
}

pub trait Size {
    /// Returns number of the elements in the underlying buffer.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn size(&self) -> usize;
}
