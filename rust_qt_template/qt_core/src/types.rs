extern crate libc;

pub use self::libc::{
  c_char, c_schar, c_uchar, c_short, c_ushort, c_int, c_uint,
  c_long, c_ulong, c_longlong, c_ulonglong, wchar_t, size_t,
  c_void
};

// TODO: qreal = f32 on some platforms

#[allow(non_camel_case_types)]
pub type qreal = f64;
