use cpp_utils::{ConstPtr, CppBox};
use std::ffi::CStr;
use crate::QObject;
use std::marker::PhantomData;
use crate::q_meta_object::Connection;

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

pub trait AsReceiver {
    type Arguments;
    fn as_receiver(self) -> Receiver<Self::Arguments>;
}

impl<A> AsReceiver for Receiver<A> {
    type Arguments = A;
    fn as_receiver(self) -> Receiver<A> {
        self
    }
}

impl<A> AsReceiver for Signal<A> {
    type Arguments = A;
    fn as_receiver(self) -> Receiver<A> {
        self.0
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

        crate::QObject::connect(
            self.0.qobject,
            ConstPtr::new(self.0.receiver_id.as_ptr()),
            receiver.qobject,
            ConstPtr::new(receiver.receiver_id.as_ptr()),
        )
    }
}
