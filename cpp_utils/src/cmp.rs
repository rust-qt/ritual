pub trait Lt<Rhs: ?Sized = Self> {
    /// This method tests less than (for `self` and `other`) and is used by the `<` operator.
    fn lt(&self, other: &Rhs) -> bool;
}

pub trait Le<Rhs: ?Sized = Self> {
    /// This method tests less than or equal to (for `self` and `other`) and is used by the `<=`
    fn le(&self, other: &Rhs) -> bool;
}

pub trait Gt<Rhs: ?Sized = Self> {
    /// This method tests greater than (for `self` and `other`) and is used by the `>` operator.
    fn gt(&self, other: &Rhs) -> bool;
}

pub trait Ge<Rhs: ?Sized = Self> {
    /// This method tests greater than or equal to (for `self` and `other`) and is used by the `>=`
    /// operator.
    fn ge(&self, other: &Rhs) -> bool;
}
