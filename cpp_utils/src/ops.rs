use crate::{
    cmp::{Ge, Gt, Le, Lt},
    CppBox, CppDeletable, MutPtr, MutRef, Ptr, Ref,
};
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

macro_rules! define_op {
    ($trait1:ident, $func:ident) => {
        impl<'a, T: CppDeletable, U> $trait1<U> for &'a CppBox<T>
        where
            &'a T: $trait1<U>,
        {
            type Output = <&'a T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                (**self).$func(rhs)
            }
        }

        impl<T: 'static, U> $trait1<U> for Ptr<T>
        where
            &'static T: $trait1<U>,
        {
            type Output = <&'static T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                unsafe { (*self.as_raw_ptr()).$func(rhs) }
            }
        }

        impl<T: 'static, U> $trait1<U> for MutPtr<T>
        where
            &'static T: $trait1<U>,
        {
            type Output = <&'static T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                unsafe { (*self.as_raw_ptr()).$func(rhs) }
            }
        }

        impl<T: 'static, U> $trait1<U> for Ref<T>
        where
            &'static T: $trait1<U>,
        {
            type Output = <&'static T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                unsafe { (*self.as_raw_ptr()).$func(rhs) }
            }
        }

        impl<T: 'static, U> $trait1<U> for MutRef<T>
        where
            &'static T: $trait1<U>,
        {
            type Output = <&'static T as $trait1<U>>::Output;

            fn $func(self, rhs: U) -> Self::Output {
                unsafe { (*self.as_raw_ptr()).$func(rhs) }
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
        impl<T: CppDeletable, U> $trait1<U> for CppBox<T>
        where
            T: $trait1<U>,
        {
            fn $func(&mut self, rhs: U) {
                (**self).$func(rhs);
            }
        }

        impl<T: 'static, U> $trait1<U> for MutPtr<T>
        where
            T: $trait1<U>,
        {
            fn $func(&mut self, rhs: U) {
                unsafe { (&mut *self.as_mut_raw_ptr()).$func(rhs) };
            }
        }

        impl<T: 'static, U> $trait1<U> for MutRef<T>
        where
            T: $trait1<U>,
        {
            fn $func(&mut self, rhs: U) {
                unsafe { (&mut *self.as_mut_raw_ptr()).$func(rhs) };
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

// TODO: Neg, Not, PartialEq, PartialOrd

macro_rules! define_comparison_op {
    ($container:ident) => {
        impl<T: CppDeletable, U> PartialEq<U> for $container<T>
        where
            T: PartialEq<U>,
        {
            fn eq(&self, rhs: &U) -> bool {
                &**self == rhs
            }
        }

        impl<T: CppDeletable, U> PartialOrd<U> for $container<T>
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
    };
}

define_comparison_op!(CppBox);
define_comparison_op!(Ptr);
define_comparison_op!(MutPtr);
define_comparison_op!(Ref);
define_comparison_op!(MutRef);
