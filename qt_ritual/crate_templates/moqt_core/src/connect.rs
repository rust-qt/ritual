use crate::q_meta_object::Connection;
use crate::QObject;
use cpp_utils::{ConstPtr, ConstRef, CppBox, CppDeletable, Ptr};
use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;

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

pub struct Receiver<Arguments> {
    q_object: ConstRef<QObject>,
    receiver_id: &'static CStr,
    _marker: PhantomData<Arguments>,
}

impl<A> Clone for Receiver<A> {
    fn clone(&self) -> Self {
        Receiver {
            q_object: self.q_object,
            receiver_id: self.receiver_id,
            _marker: PhantomData,
        }
    }
}
impl<A> Copy for Receiver<A> {}

impl<A> fmt::Debug for Receiver<A> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Receiver")
            .field("qobject", &self.q_object)
            .field("receiver_id", &self.receiver_id)
            .finish()
    }
}

impl<A> Receiver<A> {
    pub fn new(q_object: ConstRef<QObject>, receiver_id: &'static CStr) -> Self {
        Self {
            q_object,
            receiver_id,
            _marker: PhantomData,
        }
    }
}

pub struct Signal<Arguments>(Receiver<Arguments>);

impl<A> Clone for Signal<A> {
    fn clone(&self) -> Self {
        Signal(self.0)
    }
}

impl<A> Copy for Signal<A> {}

impl<A> fmt::Debug for Signal<A> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Signal")
            .field("qobject", &self.0.q_object)
            .field("receiver_id", &self.0.receiver_id)
            .finish()
    }
}

impl<A> Signal<A> {
    pub fn new(q_object: ConstRef<QObject>, receiver_id: &'static CStr) -> Self {
        Signal(Receiver::new(q_object, receiver_id))
    }
}

pub trait AsReceiver {
    type Arguments;
    fn as_receiver(&self) -> Receiver<Self::Arguments>;
}

impl<A> AsReceiver for Receiver<A> {
    type Arguments = A;
    fn as_receiver(&self) -> Receiver<A> {
        *self
    }
}

impl<A> AsReceiver for Signal<A> {
    type Arguments = A;
    fn as_receiver(&self) -> Receiver<A> {
        self.0
    }
}

impl<T> AsReceiver for Ptr<T>
where
    T: AsReceiver,
{
    type Arguments = <T as AsReceiver>::Arguments;
    fn as_receiver(&self) -> Receiver<Self::Arguments> {
        (**self).as_receiver()
    }
}

impl<T> AsReceiver for ConstRef<T>
where
    T: AsReceiver,
{
    type Arguments = <T as AsReceiver>::Arguments;
    fn as_receiver(&self) -> Receiver<Self::Arguments> {
        (**self).as_receiver()
    }
}

impl<'a, T: CppDeletable> AsReceiver for &'a CppBox<T>
where
    T: AsReceiver,
{
    type Arguments = <T as AsReceiver>::Arguments;
    fn as_receiver(&self) -> Receiver<Self::Arguments> {
        (***self).as_receiver()
    }
}

impl<SignalArguments> Signal<SignalArguments> {
    pub unsafe fn connect<R>(&self, receiver: R) -> CppBox<Connection>
    where
        R: AsReceiver,
        SignalArguments: ArgumentsCompatible<R::Arguments>,
    {
        let receiver = receiver.as_receiver();
        // TODO: allow to change connection type
        // TODO: meta_object::Connection should have operator bool()

        crate::QObject::connect_4a(
            self.0.q_object.into(),
            ConstPtr::from_raw(self.0.receiver_id.as_ptr()),
            receiver.q_object.into(),
            ConstPtr::from_raw(receiver.receiver_id.as_ptr()),
        )
    }
}
