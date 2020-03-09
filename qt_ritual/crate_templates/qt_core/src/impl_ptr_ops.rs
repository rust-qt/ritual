use crate::{QBox, QMutPtr, QObject, QPtr};
use cpp_core::{
    cmp::{Ge, Gt, Le, Lt},
    CppDeletable, StaticUpcast,
};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

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
        impl<'a, T, U> $trait1<U> for &'a QMutPtr<T>
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

macro_rules! define_assign_op {
    ($trait1:ident, $func:ident) => {
        impl<T, U> $trait1<U> for QBox<T>
        where
            T: $trait1<U> + CppDeletable + StaticUpcast<QObject>,
        {
            fn $func(&mut self, rhs: U) {
                (**self).$func(rhs);
            }
        }
        impl<T, U> $trait1<U> for QMutPtr<T>
        where
            T: $trait1<U> + StaticUpcast<QObject>,
        {
            fn $func(&mut self, rhs: U) {
                (**self).$func(rhs);
            }
        }
    };
}

define_assign_op!(AddAssign, add_assign);
define_assign_op!(SubAssign, sub_assign);
define_assign_op!(MulAssign, mul_assign);
define_assign_op!(DivAssign, div_assign);
define_assign_op!(RemAssign, rem_assign);
define_assign_op!(BitAndAssign, bitand_assign);
define_assign_op!(BitOrAssign, bitor_assign);
define_assign_op!(BitXorAssign, bitxor_assign);
define_assign_op!(ShlAssign, shl_assign);
define_assign_op!(ShrAssign, shr_assign);

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

impl<T, U> PartialEq<U> for QMutPtr<T>
where
    T: PartialEq<U> + StaticUpcast<QObject>,
{
    fn eq(&self, rhs: &U) -> bool {
        &**self == rhs
    }
}

impl<T, U> PartialOrd<U> for QMutPtr<T>
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
