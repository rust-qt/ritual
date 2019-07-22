/// Used to do value-to-value conversions while consuming the input value.
pub trait CastFrom<T>: Sized {
    /// Performs the conversion.
    fn from(_: T) -> Self;
}

pub trait CastInto<T>: Sized {
    fn into(self) -> T;
}

impl<T, U: CastFrom<T>> CastInto<U> for T {
    fn into(self) -> U {
        U::from(self)
    }
}
