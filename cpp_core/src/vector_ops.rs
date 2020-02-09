//! Traits for common operations on C++ vectors.

use crate::{MutPtr, Ptr};
use std::slice;

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
    unsafe fn data_mut(&mut self) -> Self::Output;
}

pub trait Size {
    /// Returns size of the underlying buffer in elements.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn size(&self) -> usize;
}

pub trait VectorAsSlice {
    type Item;
    /// Returns the content of the vector as a slice.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn vector_as_slice(&self) -> &[Self::Item];
}

pub trait VectorAsMutSlice {
    type Item;
    /// Returns the content of the vector as a slice.
    ///
    /// # Safety
    ///
    /// The caller must make sure `self` contains a valid pointer. The content must
    /// not be read or modified through other ways while the returned slice exists.
    /// This function
    /// may invoke arbitrary foreign code, so no safety guarantees can be made.
    unsafe fn vector_as_mut_slice(&mut self) -> &mut [Self::Item];
}

impl<V, T> VectorAsSlice for V
where
    V: Data<Output = Ptr<T>> + Size,
{
    type Item = T;
    unsafe fn vector_as_slice(&self) -> &[T] {
        let ptr = self.data().as_raw_ptr();
        let size = self.size();
        slice::from_raw_parts(ptr, size)
    }
}

impl<V, T> VectorAsMutSlice for V
where
    V: DataMut<Output = MutPtr<T>> + Size,
{
    type Item = T;
    unsafe fn vector_as_mut_slice(&mut self) -> &mut [T] {
        let ptr = self.data_mut().as_mut_raw_ptr();
        let size = self.size();
        slice::from_raw_parts_mut(ptr, size)
    }
}
