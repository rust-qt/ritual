use crate::QString;
use cpp_core::CppBox;
use std::os::raw::{c_char, c_int};

/// Allows to convert Qt strings to `std` strings
impl<'a> From<&'a QString> for String {
    fn from(s: &'a QString) -> String {
        s.to_std_string()
    }
}

impl QString {
    /// Creates Qt string from an `std` string.
    ///
    /// `QString` makes a deep copy of the data.
    pub fn from_std_str<S: AsRef<str>>(s: S) -> CppBox<QString> {
        let slice = s.as_ref().as_bytes();
        unsafe { QString::from_utf8_char_int(slice.as_ptr() as *mut c_char, slice.len() as c_int) }
    }

    /// Creates an `std` string from a Qt string.
    pub fn to_std_string(&self) -> String {
        unsafe {
            let buf = self.to_utf8();
            let bytes =
                std::slice::from_raw_parts(buf.const_data() as *const u8, buf.size() as usize);
            std::str::from_utf8_unchecked(bytes).to_string()
        }
    }
}

/// Creates a `QString` from a Rust string.
///
/// This is the same as `QString::from_std_str(str)`.
pub fn qs<S: AsRef<str>>(str: S) -> CppBox<QString> {
    QString::from_std_str(str)
}
