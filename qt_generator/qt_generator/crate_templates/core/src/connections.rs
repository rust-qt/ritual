use libc::c_char;

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

/// An object that can be connected to a Qt signal.
///
/// Both Qt signals and slots can act as receivers, i.e. there can be
/// signal-to-signal and signal-to-slot connections.
pub trait Receiver {
  /// Tuple of argument types of this receiver.
  type Arguments;
  /// Returns reference to the `QObject` that owns this signal or slot.
  fn object(&self) -> &::object::Object;
  /// Returns a null-terminated Latin-1 string that identifies
  /// this signal or slot within the owning `QObject` in Qt system.
  fn receiver_id() -> &'static [u8];
}

/// A Qt signal.
pub trait Signal : Receiver {

  /// Connects this signal to another signal or slot with compatible arguments.
  fn connect<A, R: Receiver<Arguments=A>>(&self, receiver: &R) -> ::meta_object::Connection
  where Self::Arguments: ArgumentsCompatible<A>
  {
    // TODO: allow to change connection type
    // TODO: meta_object::Connection should have operator bool()
    ::object::Object::connect_static((
      self.object() as *const ::object::Object,
      Self::receiver_id().as_ptr() as *const c_char,
      receiver.object() as *const ::object::Object,
      R::receiver_id().as_ptr() as *const c_char,
      ::cpp_utils::AsStruct
    ))
  }
}

