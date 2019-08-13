use crate::ops::{Increment, Indirection};
use crate::{CppBox, CppDeletable, Ref};

pub struct CppIterator<T1, T2>
where
    T1: CppDeletable,
    T2: CppDeletable,
{
    current: CppBox<T1>,
    end: CppBox<T2>,
}

pub unsafe fn cpp_iter<T1, T2>(begin: CppBox<T1>, end: CppBox<T2>) -> CppIterator<T1, T2>
where
    T1: CppDeletable,
    T2: CppDeletable,
{
    CppIterator {
        current: begin,
        end,
    }
}

impl<T1, T2> Iterator for CppIterator<T1, T2>
where
    T1: CppDeletable + PartialEq<Ref<T2>> + 'static,
    T2: CppDeletable,
    for<'a> &'a T1: Indirection,
    for<'a> &'a mut T1: Increment,
{
    type Item = <&'static T1 as Indirection>::Output;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.current == self.end.as_ref() {
                None
            } else {
                let inner = &*self.current.as_raw_ptr();
                let value = inner.indirection();
                self.current.inc();
                Some(value)
            }
        }
    }
}
