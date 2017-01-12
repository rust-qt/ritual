use std;
extern crate libc;
use self::libc::c_int;

#[derive(Clone, Copy)]
pub struct Flags<E> {
  value: c_int,
  _phantom_data: std::marker::PhantomData<E>,
}

impl<E: FlaggableEnum> Flags<E> {
  pub fn from_int(value: c_int) -> Self {
    Flags {
      value: value,
      _phantom_data: std::marker::PhantomData,
    }
  }
  pub fn from_enum(value: E) -> Self {
    Self::from_int(value.to_flag_value())
  }
  pub fn to_int(self) -> c_int {
    self.value
  }
  pub fn test_flag(self, flag: E) -> bool {
    self.value & flag.to_flag_value() != 0
  }
  pub fn is_empty(self) -> bool {
    self.value == 0
  }
}



impl<E: FlaggableEnum, T: EnumOrFlags<E>> std::ops::BitOr<T> for Flags<E> {
  type Output = Flags<E>;
  fn bitor(self, rhs: T) -> Flags<E> {
    let mut r = self.clone();
    r.value |= rhs.to_flags().to_int();
    r
  }
}

impl<E: FlaggableEnum, T: EnumOrFlags<E>> std::ops::BitAnd<T> for Flags<E> {
  type Output = Flags<E>;
  fn bitand(self, rhs: T) -> Flags<E> {
    let mut r = self.clone();
    r.value &= rhs.to_flags().to_int();
    r
  }
}

impl<E: FlaggableEnum, T: EnumOrFlags<E>> std::ops::BitXor<T> for Flags<E> {
  type Output = Flags<E>;
  fn bitxor(self, rhs: T) -> Flags<E> {
    let mut r = self.clone();
    r.value ^= rhs.to_flags().to_int();
    r
  }
}

pub trait FlaggableEnum: Sized + Clone {
  fn to_flag_value(self) -> c_int;
  fn enum_name() -> &'static str;
}


pub trait EnumOrFlags<T> {
  fn to_flags(self) -> Flags<T>;
}

impl<T> EnumOrFlags<T> for Flags<T>
  where T: Clone
{
  fn to_flags(self) -> Flags<T> {
    self.clone()
  }
}

impl<T: FlaggableEnum> std::fmt::Debug for Flags<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "Flags<{}>({})", T::enum_name(), self.value)
  }
}

impl<T> Default for Flags<T> {
  fn default() -> Self {
    Flags {
      value: 0,
      _phantom_data: std::marker::PhantomData,
    }
  }
}
