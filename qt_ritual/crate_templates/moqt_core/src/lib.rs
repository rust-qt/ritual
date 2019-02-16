use std::os::raw::c_int;

/// Rust alternative to Qt's `QFlags` types.
///
/// `Flags<E>` is an OR-combination of integer values of the enum type `E`.
#[derive(Clone, Copy)]
pub struct QFlags<E: QFlaggableEnum> {
    value: c_int,
    _phantom_data: std::marker::PhantomData<E>,
}

impl<E: QFlaggableEnum> QFlags<E> {
    /// Converts integer `value` to `Flags`.
    pub fn from_int(value: c_int) -> Self {
        QFlags {
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

impl<E: QFlaggableEnum, T: EnumOrFlags<E>> std::ops::BitOr<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitor(self, rhs: T) -> QFlags<E> {
        let mut r = self.clone();
        r.value |= rhs.to_flags().to_int();
        r
    }
}

impl<E: QFlaggableEnum, T: EnumOrFlags<E>> std::ops::BitAnd<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitand(self, rhs: T) -> QFlags<E> {
        let mut r = self.clone();
        r.value &= rhs.to_flags().to_int();
        r
    }
}

impl<E: QFlaggableEnum, T: EnumOrFlags<E>> std::ops::BitXor<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitxor(self, rhs: T) -> QFlags<E> {
        let mut r = self.clone();
        r.value ^= rhs.to_flags().to_int();
        r
    }
}

/// Enum type with values suitable for constructing OR-combinations for `Flags`.
pub trait QFlaggableEnum: Sized + Clone {
    /// Returns integer value of this enum variant.
    fn to_flag_value(self) -> c_int;
    /// Returns name of the type for debug output.
    fn enum_name() -> &'static str;
}

/// Trait representing types that can be converted to `Flags`.
pub trait EnumOrFlags<T: QFlaggableEnum> {
    fn to_flags(self) -> QFlags<T>;
}
// TODO: use Into and From traits instead

impl<T: QFlaggableEnum> EnumOrFlags<T> for QFlags<T>
where
    T: Clone,
{
    fn to_flags(self) -> QFlags<T> {
        self.clone()
    }
}

impl<T: QFlaggableEnum> std::fmt::Debug for QFlags<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "QFlags<{}>({})", T::enum_name(), self.value)
    }
}

impl<T: QFlaggableEnum> Default for QFlags<T> {
    fn default() -> Self {
        QFlags {
            value: 0,
            _phantom_data: std::marker::PhantomData,
        }
    }
}
