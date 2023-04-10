use crate::QByteArray;
use cpp_core::CppBox;
use std::os::raw::{c_char, c_int};

impl QByteArray {
    /// Creates a `QByteArray` containing bytes from `slice`.
    ///
    /// `QByteArray` makes a deep copy of the data.
    pub unsafe fn from_slice(slice: &[u8]) -> CppBox<QByteArray> {
        QByteArray::from_char_int(slice.as_ptr() as *const c_char, slice.len() as c_int)
    }
}
