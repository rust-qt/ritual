use crate::{QBox, QObject, QPtr};
use cpp_core::{
    cmp::{Ge, Gt, Le, Lt},
    CppDeletable, StaticUpcast,
};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub};

macro_rules! define_op {
    ($trait1:ident, $func:ident) => {
        impl<'a, T, U> $trait1<U> for &'a QBox<T>
        where
            T: CppDeletable + StaticUpcast<QObject>,
            &'a T: $trait1<U>,
        {
            type Output = <&'a T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                (**self).$func(rhs)
            }
        }
        impl<'a, T, U> $trait1<U> for &'a QPtr<T>
        where
            T: StaticUpcast<QObject>,
            &'a T: $trait1<U>,
        {
            type Output = <&'a T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                (**self).$func(rhs)
            }
        }
    };
}

define_op!(Add, add);
define_op!(Sub, sub);
define_op!(Mul, mul);
define_op!(Div, div);
define_op!(Rem, rem);
define_op!(BitAnd, bitand);
define_op!(BitOr, bitor);
define_op!(BitXor, bitxor);
define_op!(Shl, shl);
define_op!(Shr, shr);

impl<T, U> PartialEq<U> for QBox<T>
where
    T: PartialEq<U> + CppDeletable + StaticUpcast<QObject>,
{
    fn eq(&self, rhs: &U) -> bool {
        &**self == rhs
    }
}

impl<T, U> PartialOrd<U> for QBox<T>
where
    T: Lt<U> + Le<U> + Gt<U> + Ge<U> + PartialEq<U> + CppDeletable + StaticUpcast<QObject>,
{
    fn partial_cmp(&self, other: &U) -> Option<Ordering> {
        unsafe {
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
    }

    fn lt(&self, other: &U) -> bool {
        unsafe { (**self).lt(other) }
    }

    fn le(&self, other: &U) -> bool {
        unsafe { (**self).le(other) }
    }

    fn gt(&self, other: &U) -> bool {
        unsafe { (**self).gt(other) }
    }

    fn ge(&self, other: &U) -> bool {
        unsafe { (**self).ge(other) }
    }
}

impl<T, U> PartialEq<U> for QPtr<T>
where
    T: PartialEq<U> + StaticUpcast<QObject>,
{
    fn eq(&self, rhs: &U) -> bool {
        &**self == rhs
    }
}

impl<T, U> PartialOrd<U> for QPtr<T>
where
    T: Lt<U> + Le<U> + Gt<U> + Ge<U> + PartialEq<U> + StaticUpcast<QObject>,
{
    fn partial_cmp(&self, other: &U) -> Option<Ordering> {
        unsafe {
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
    }

    fn lt(&self, other: &U) -> bool {
        unsafe { (**self).lt(other) }
    }

    fn le(&self, other: &U) -> bool {
        unsafe { (**self).le(other) }
    }

    fn gt(&self, other: &U) -> bool {
        unsafe { (**self).gt(other) }
    }

    fn ge(&self, other: &U) -> bool {
        unsafe { (**self).ge(other) }
    }
}
