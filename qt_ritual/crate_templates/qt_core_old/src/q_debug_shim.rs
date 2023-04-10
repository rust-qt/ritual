use crate::{QDebug, QString};
use std::fmt;
use std::ops::Shl;

/// Provides a `std::fmt::Debug` implementation for types with a `QDebug` operator.
///
/// Use `qdbg` function instead of using this type directly.
pub struct QDebugShim<T>(T);

impl<T> fmt::Debug for QDebugShim<T>
where
    T: Copy,
    for<'a> &'a QDebug: Shl<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let string = QString::new();
            let q_debug = QDebug::from_q_string(&string);
            let _ = &q_debug << self.0;
            drop(q_debug);
            write!(f, "{}", string.to_std_string())
        }
    }
}

/// Returns an object that implements `std::fmt::Debug` for a `value`
/// that has a `QDebug` operator.
///
/// Example:
/// ```
/// use qt_core::{qdbg, QVectorOfInt};
/// # unsafe {
/// let x = QVectorOfInt::new_0a();
/// x.append_int(&1);
/// x.append_int(&2);
/// println!("{:?}", qdbg(x.as_ref()));
/// # }
/// ```
pub unsafe fn qdbg<T>(value: T) -> QDebugShim<T> {
    QDebugShim(value)
}
