use libc::c_int;
use std;

/// Rust alternative to Qt's `QFlags` types.
///
/// `Flags<E>` is an OR-combination of integer values of the enum type `E`.
#[derive(Clone, Copy)]
pub struct Flags<E: FlaggableEnum> {
  value: c_int,
  _phantom_data: std::marker::PhantomData<E>,
}

impl<E: FlaggableEnum> Flags<E> {
  /// Converts integer `value` to `Flags`.
  pub fn from_int(value: c_int) -> Self {
    Flags {
      value: value,
      _phantom_data: std::marker::PhantomData,
    }
  }
  /// Converts `value` to `Flags` containing that single value.
  pub fn from_enum(value: E) -> Self {
    Self::from_int(value.to_flag_value())
  }
  /// Converts `Flags` to integer.
  pub fn to_int(self) -> c_int {
    self.value
  }
  /// Returns `true` if `flag` is enabled in `self`.
  pub fn test_flag(self, flag: E) -> bool {
    self.value & flag.to_flag_value() != 0
  }
  /// Returns `true` if this value has no flags enabled.
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

/// Enum type with values suitable for constructing OR-combinations for `Flags`.
pub trait FlaggableEnum: Sized + Clone {
  /// Returns integer value of this enum variant.
  fn to_flag_value(self) -> c_int;
  /// Returns name of the type for debug output.
  fn enum_name() -> &'static str;
}

/// Trait representing types that can be converted to `Flags`.
pub trait EnumOrFlags<T: FlaggableEnum> {
  fn to_flags(self) -> Flags<T>;
}
// TODO: use Into and From traits instead

impl<T: FlaggableEnum> EnumOrFlags<T> for Flags<T>
where
  T: Clone,
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

impl<T: FlaggableEnum> Default for Flags<T> {
  fn default() -> Self {
    Flags {
      value: 0,
      _phantom_data: std::marker::PhantomData,
    }
  }
}
