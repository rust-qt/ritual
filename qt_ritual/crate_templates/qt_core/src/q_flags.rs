use std::fmt;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, BitXor};
use std::os::raw::c_int;

/// An OR-combination of integer values of the enum type `E`.
///
/// This type serves as a replacement for Qt's `QFlags` C++ template class.
#[derive(Clone, Copy)]
pub struct QFlags<E> {
    value: c_int,
    _phantom_data: PhantomData<E>,
}

impl<E> From<c_int> for QFlags<E> {
    fn from(value: c_int) -> Self {
        Self {
            value,
            _phantom_data: PhantomData,
        }
    }
}

impl<E> From<QFlags<E>> for c_int {
    fn from(flags: QFlags<E>) -> Self {
        flags.value
    }
}

impl<E> QFlags<E> {
    pub fn to_int(self) -> c_int {
        self.value
    }
}

impl<E: Into<QFlags<E>>> QFlags<E> {
    /// Returns `true` if `flag` is enabled in `self`.
    pub fn test_flag(self, flag: E) -> bool {
        self.value & flag.into().value != 0
    }

    /// Returns `true` if this value has no flags enabled.
    pub fn is_empty(self) -> bool {
        self.value == 0
    }
}

impl<E, T: Into<QFlags<E>>> BitOr<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitor(self, rhs: T) -> QFlags<E> {
        Self {
            value: self.value | rhs.into().value,
            _phantom_data: PhantomData,
        }
    }
}

impl<E, T: Into<QFlags<E>>> BitAnd<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitand(self, rhs: T) -> QFlags<E> {
        Self {
            value: self.value & rhs.into().value,
            _phantom_data: PhantomData,
        }
    }
}

impl<E, T: Into<QFlags<E>>> BitXor<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitxor(self, rhs: T) -> QFlags<E> {
        Self {
            value: self.value ^ rhs.into().value,
            _phantom_data: PhantomData,
        }
    }
}

impl<E> Default for QFlags<E> {
    fn default() -> Self {
        QFlags {
            value: 0,
            _phantom_data: PhantomData,
        }
    }
}

impl<T> fmt::Debug for QFlags<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "QFlags({})", self.value)
    }
}
