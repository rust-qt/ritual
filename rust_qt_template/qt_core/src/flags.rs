extern crate std;

use types::c_int;

#[derive(Clone, Copy)]
pub struct QFlags<E> {
  value: c_int,
  _phantom_data: std::marker::PhantomData<E>,
}

impl<E: FlaggableEnum> QFlags<E> {
  pub fn from_int(value: c_int) -> Self {
    QFlags {
      value: value,
      _phantom_data: std::marker::PhantomData,
    }
  }
  pub fn from_enum(value: E) -> Self {
    Self::from_int(value.to_int())
  }
  pub fn to_int(self) -> c_int {
    self.value
  }
  pub fn test_flag(self, flag: E) -> bool {
    self.value & flag.to_int() != 0
  }
  pub fn is_empty(self) -> bool {
    return self.value == 0
  }

}



impl<E: FlaggableEnum, T: EnumOrFlags<E>> std::ops::BitOr<T> for QFlags<E> {
  type Output = QFlags<E>;
  fn bitor(self, rhs: T) -> QFlags<E> {
    let mut r = self.clone();
    r.value |= rhs.to_flags().to_int();
    r
  }
}

impl<E: FlaggableEnum, T: EnumOrFlags<E>> std::ops::BitAnd<T> for QFlags<E> {
  type Output = QFlags<E>;
  fn bitand(self, rhs: T) -> QFlags<E> {
    let mut r = self.clone();
    r.value &= rhs.to_flags().to_int();
    r
  }
}

impl<E: FlaggableEnum, T: EnumOrFlags<E>> std::ops::BitXor<T> for QFlags<E> {
  type Output = QFlags<E>;
  fn bitxor(self, rhs: T) -> QFlags<E> {
    let mut r = self.clone();
    r.value ^= rhs.to_flags().to_int();
    r
  }
}

pub trait FlaggableEnum : Sized + Clone {
  fn to_int(self) -> c_int;
  fn enum_name() -> &'static str;
}


pub trait EnumOrFlags<T> {
  fn to_flags(self) -> QFlags<T>;
}

impl<T> EnumOrFlags<T> for QFlags<T>
  where T: Clone
{
  fn to_flags(self) -> QFlags<T> {
    self.clone()
  }
}

impl<T: FlaggableEnum> std::fmt::Debug for QFlags<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "QFlags<{}>({})", T::enum_name(), self.value)
  }
}

impl<T> Default for QFlags<T> {
  fn default() -> Self {
    QFlags {
      value: 0,
      _phantom_data: std::marker::PhantomData
    }
  }
}
