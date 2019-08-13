pub trait Increment {
    type Output;

    fn inc(self) -> Self::Output;
}

pub trait Indirection {
    type Output;

    fn indirection(self) -> Self::Output;
}
