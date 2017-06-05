include_generated!();

use libc::{c_char, c_int};
use std;

/// Allows to convert built-in strings `&str` to Qt strings
impl<'a> From<&'a str> for ::string::String {
  fn from(s: &'a str) -> ::string::String {
    ::string::String::from_std_str(s)
  }
}

/// Allows to convert Qt strings to `std` strings
impl<'a> From<&'a ::string::String> for ::std::string::String {
  fn from(s: &'a ::string::String) -> ::std::string::String {
    s.to_std_string()
  }
}

impl ::string::String {
  /// Creates Qt string from an `std` string.
  pub fn from_std_str<S: AsRef<str>>(s: S) -> ::string::String {
    let slice = s.as_ref().as_bytes();
    unsafe {
      ::string::String::from_utf8_unsafe((slice.as_ptr() as *const c_char, slice.len() as c_int))
    }
  }

  /// Creates `std` string from a Qt string.
  pub fn to_std_string(&self) -> std::string::String {
    let buf = self.to_utf8();
    unsafe {
      let bytes = std::slice::from_raw_parts(buf.const_data() as *const u8, buf.count(()) as usize);
      std::str::from_utf8_unchecked(bytes).to_string()
    }
  }
}
