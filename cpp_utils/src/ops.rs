use crate::{
    cmp::{Ge, Gt, Le, Lt},
    CppBox, CppDeletable,
};
use std::cmp::{Ordering, PartialEq, PartialOrd};
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

impl<T: CppDeletable, U> PartialEq<U> for CppBox<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, rhs: &U) -> bool {
        &**self == rhs
    }
}

impl<T: CppDeletable, U> PartialOrd<U> for CppBox<T>
where
    T: Lt<U> + Le<U> + Gt<U> + Ge<U> + PartialEq<U>,
{
    fn partial_cmp(&self, other: &U) -> Option<Ordering> {
        if &**self == other {
            Some(Ordering::Equal)
        } else if (**self).lt(other) {
            Some(Ordering::Less)
        } else if (**self).gt(other) {
            Some(Ordering::Greater)
        } else {
            None
        }
    }

    fn lt(&self, other: &U) -> bool {
        (**self).lt(other)
    }

    fn le(&self, other: &U) -> bool {
        (**self).le(other)
    }

    fn gt(&self, other: &U) -> bool {
        (**self).gt(other)
    }

    fn ge(&self, other: &U) -> bool {
        (**self).ge(other)
    }
}
