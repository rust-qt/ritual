use crate::QString;
use cpp_utils::{ConstPtr, CppBox};
use std::os::raw::{c_char, c_int};

/// Allows to convert Qt strings to `std` strings
impl<'a> From<&'a QString> for String {
    fn from(s: &'a QString) -> String {
        s.to_std_string()
    }
}

impl QString {
    /// Creates Qt string from an `std` string.
    pub fn from_std_str<S: AsRef<str>>(s: S) -> CppBox<QString> {
        let slice = s.as_ref().as_bytes();
        unsafe {
            QString::from_utf8_2a(
                ConstPtr::from_raw(slice.as_ptr() as *const c_char),
                slice.len() as c_int,
            )
        }
    }

    /// Creates `std` string from a Qt string.
    pub fn to_std_string(&self) -> String {
        unsafe {
            let buf = self.to_utf8();
            let bytes = std::slice::from_raw_parts(
                buf.const_data().as_raw_ptr() as *const u8,
                buf.size() as usize,
            );
            std::str::from_utf8_unchecked(bytes).to_string()
        }
    }
}
