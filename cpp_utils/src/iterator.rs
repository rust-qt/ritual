use crate::ops::{Decrement, Increment, Indirection};
use crate::{CppBox, CppDeletable, Ref};

pub struct CppIterator<T1, T2>
where
    T1: CppDeletable,
    T2: CppDeletable,
{
    begin: CppBox<T1>,
    end: CppBox<T2>,
}

pub unsafe fn cpp_iter<T1, T2>(begin: CppBox<T1>, end: CppBox<T2>) -> CppIterator<T1, T2>
where
    T1: CppDeletable,
    T2: CppDeletable,
{
    CppIterator { begin, end }
}

impl<T1, T2> Iterator for CppIterator<T1, T2>
where
    T1: CppDeletable + PartialEq<Ref<T2>> + 'static,
    T2: CppDeletable,
    &'static T1: Indirection,
    &'static mut T1: Increment,
{
    type Item = <&'static T1 as Indirection>::Output;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.begin == self.end.as_ref() {
                None
            } else {
                let inner = &mut *self.begin.as_mut_raw_ptr();
                let value = inner.indirection();
                let inner = &mut *self.begin.as_mut_raw_ptr();
                inner.inc();
                Some(value)
            }
        }
    }
}

impl<T1, T2> DoubleEndedIterator for CppIterator<T1, T2>
where
    T1: CppDeletable + PartialEq<Ref<T2>> + 'static,
    T2: CppDeletable + 'static,
    &'static T1: Indirection,
    &'static mut T1: Increment,
    &'static mut T2: Decrement,
    &'static T2: Indirection<Output = <&'static T1 as Indirection>::Output>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.begin == self.end.as_ref() {
                None
            } else {
                let inner = &mut *self.end.as_mut_raw_ptr();
                inner.dec();
                let inner = &mut *self.end.as_mut_raw_ptr();
                let value = inner.indirection();
                Some(value)
            }
        }
    }
}
