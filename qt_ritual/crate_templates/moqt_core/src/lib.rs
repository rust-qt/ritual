use std::os::raw::c_int;
use std::ffi::CStr;
use std::marker::PhantomData;
use cpp_utils::ConstPtr;

/// Rust alternative to Qt's `QFlags` types.
///
/// `Flags<E>` is an OR-combination of integer values of the enum type `E`.
#[derive(Clone, Copy)]
pub struct QFlags<E> {
    value: c_int,
    _phantom_data: std::marker::PhantomData<E>,
}

impl<E> From<c_int> for QFlags<E> {
    fn from(value: c_int) -> Self {
        Self {
            value,
            _phantom_data: std::marker::PhantomData,
        }
    }
}

impl<E> From<QFlags<E>> for c_int {
    fn from(flags: QFlags<E>) -> Self {
        flags.value
    }
}

impl<E> QFlags<E> {
    pub fn to_int(self) -> c_int {
        self.value
    }
}

impl<E: Into<QFlags<E>>> QFlags<E> {
    /// Returns `true` if `flag` is enabled in `self`.
    pub fn test_flag(self, flag: E) -> bool {
        self.value & flag.into().value != 0
    }

    /// Returns `true` if this value has no flags enabled.
    pub fn is_empty(self) -> bool {
        self.value == 0
    }
}

impl<E, T: Into<QFlags<E>>> std::ops::BitOr<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitor(self, rhs: T) -> QFlags<E> {
        Self {
            value: self.value | rhs.into().value,
            _phantom_data: std::marker::PhantomData,
        }
    }
}

/*
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
*/

impl<E> Default for QFlags<E> {
    fn default() -> Self {
        QFlags {
            value: 0,
            _phantom_data: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for QFlags<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "QFlags({})", self.value)
    }
}

/// Argument types compatible for signal connection.
///
/// Qt allows to connect senders to receivers if their argument types are the same.
/// Additionally, Qt allows receivers to have fewer arguments than the sender.
/// Other arguments are simply omitted in such a connection.
///
/// Note that Qt also allows to connect senders to receivers when their argument types
/// are not the same but there is a conversion from sender's argument types
/// to receiver's corresponding argument types. This ability is not exposed in Rust
/// wrapper's API.
///
/// Argument types are expressed as a tuple.
/// `ArgumentsCompatible<T1>` is implemented for `T2` tuple if
/// `T1` tuple can be constructed by removing some elements from the end of `T2`.
///
/// For instance, `ArgumentsCompatible<T>` and `ArgumentsCompatible<()>` are implemented
/// for every `T`.
///
/// `ArgumentsCompatible` is implemented for tuples with up to 16 items.
pub trait ArgumentsCompatible<T> {}

#[derive(Clone, Copy, Debug)]
pub struct Receiver<Arguments> {
    qobject: ConstPtr<QObject>,
    receiver_id: &'static CStr,
    _marker: PhantomData<Arguments>,
}

impl<A> Receiver<A> {
    pub fn new(qobject: ConstPtr<QObject>, receiver_id: &'static CStr) -> Self {
        Self {
            qobject: qobject,
            receiver_id,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Signal<Arguments>(Receiver<Arguments>);

impl<A> Signal<A> {
    pub fn new(qobject: ConstPtr<QObject>, receiver_id: &'static CStr) -> Self {
        Signal(Receiver::new(qobject, receiver_id))
    }
}

pub trait AsReceiver<Arguments> {
    fn as_receiver(self) -> Receiver<Arguments>;
}

impl<A> AsReceiver<A> for Receiver<A> {
    fn as_receiver(self) -> Receiver<A> {
        self
    }
}

impl<A> AsReceiver<A> for Signal<A> {
    fn as_receiver(self) -> Receiver<A> {
        self.0
    }
}

impl<SignalArguments> Signal<SignalArguments> {
    pub unsafe fn connect<ReceiverArguments, R>(&self, receiver: R) -> crate::q_meta_object::Connection
        where
            R: AsReceiver<ReceiverArguments>,
            SignalArguments: ArgumentsCompatible<ReceiverArguments>,
    {
        let receiver = receiver.as_receiver();
        // TODO: allow to change connection type
        // TODO: meta_object::Connection should have operator bool()

        crate::QObject::connect(
            self.0.qobject,
            ConstPtr::new(self.0.receiver_id.as_ptr()),
            receiver.qobject,
            ConstPtr::new(receiver.receiver_id.as_ptr()),
        )
    }
}

mod impl_arguments_compatible;
