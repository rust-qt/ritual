extern crate libc;

pub use self::libc::{
  c_char, c_schar, c_uchar, c_short, c_ushort, c_int, c_uint,
  c_long, c_ulong, c_longlong, c_ulonglong, wchar_t, size_t
};

// TODO: qreal = f32 on some platforms
pub type qreal = f64;
pub struct QFlags;
