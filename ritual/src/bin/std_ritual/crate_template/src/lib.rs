use cpp_core::{MutPtr, Ptr};
use std::slice;

pub mod vector_ops {
    pub trait Data {
        type Output;
        unsafe fn data(&self) -> Self::Output;
    }

    pub trait DataMut {
        type Output;
        unsafe fn data_mut(&mut self) -> Self::Output;
    }

    pub trait Size {
        unsafe fn size(&self) -> usize;
    }
}

pub trait VectorAsSlice {
    type Item;
    unsafe fn as_slice(&self) -> &[Self::Item];
}

pub trait VectorAsMutSlice {
    type Item;
    unsafe fn as_mut_slice(&mut self) -> &mut [Self::Item];
}

impl<V, T> VectorAsSlice for V
where
    V: vector_ops::Data<Output = Ptr<T>> + vector_ops::Size,
{
    type Item = T;
    unsafe fn as_slice(&self) -> &[T] {
        let ptr = self.data().as_raw_ptr();
        let size = self.size();
        slice::from_raw_parts(ptr, size)
    }
}

impl<V, T> VectorAsMutSlice for V
where
    V: vector_ops::DataMut<Output = MutPtr<T>> + vector_ops::Size,
{
    type Item = T;
    unsafe fn as_mut_slice(&mut self) -> &mut [T] {
        let ptr = self.data_mut().as_mut_raw_ptr();
        let size = self.size();
        slice::from_raw_parts_mut(ptr, size)
    }
}
