use crate::{CppBox, CppDeletable};
use std::ops::{Add, AddAssign};

impl<'a, T: CppDeletable, U> Add<U> for &'a CppBox<T>
where
    &'a T: Add<U>,
{
    type Output = <&'a T as Add<U>>::Output;

    fn add(self, rhs: U) -> Self::Output {
        &**self + rhs
    }
}

impl<T: CppDeletable, U> AddAssign<U> for CppBox<T>
where
    T: AddAssign<U>,
{
    fn add_assign(&mut self, rhs: U) {
        **self += rhs;
    }
}
