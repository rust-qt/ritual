pub trait Increment {
    type Output;

    fn inc(self) -> Self::Output;
}

pub trait Decrement {
    type Output;

    fn dec(self) -> Self::Output;
}

pub trait Indirection {
    type Output;

    fn indirection(self) -> Self::Output;
}

pub trait Begin {
    type Output;
    fn begin(self) -> Self::Output;
}
pub trait BeginMut {
    type Output;
    fn begin_mut(self) -> Self::Output;
}
pub trait End {
    type Output;
    fn end(self) -> Self::Output;
}
pub trait EndMut {
    type Output;
    fn end_mut(self) -> Self::Output;
}
