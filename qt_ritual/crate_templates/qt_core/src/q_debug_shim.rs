use crate::{QDebug, QString};
use std::fmt;
use std::ops::Shl;

pub struct QDebugShim<T>(T);

impl<T> fmt::Debug for QDebugShim<T>
where
    T: Copy,
    for<'a> &'a QDebug: Shl<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut string = QString::new();
            let q_debug = QDebug::from_q_string(string.as_mut_ptr());
            let _ = &q_debug << self.0;
            drop(q_debug);
            write!(f, "{}", string.to_std_string())
        }
    }
}

pub unsafe fn qdebug<T>(value: T) -> QDebugShim<T> {
    QDebugShim(value)
}
