use ::cpp_utils::AsStruct;
use ::libc::{c_char, c_int};
use ::std;

impl<'a> From<&'a str> for ::string::String {
  fn from(s: &'a str) -> ::string::String {
    ::string::String::from_std_str(s)
  }
}

impl<'a> From<&'a ::string::String> for ::std::string::String {
  fn from(s: &'a ::string::String) -> ::std::string::String {
    s.to_std_string()
  }
}

impl ::string::String {
  pub fn from_std_str<S: AsRef<str>>(s: S) -> ::string::String {
    let slice = s.as_ref().as_bytes();
    ::string::String::from_utf8((slice.as_ptr() as *const c_char, slice.len() as c_int, AsStruct))
  }

  pub fn to_std_string(&self) -> std::string::String {
    let buf = self.to_utf8(::cpp_utils::AsStruct);
    unsafe {
      let bytes = std::slice::from_raw_parts(buf.const_data() as *const u8, buf.count(()) as usize);
      std::str::from_utf8_unchecked(bytes).to_string()
    }
  }
}
