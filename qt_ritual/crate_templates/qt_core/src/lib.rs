use ::std::os::raw::c_int;
use ::std::ffi::CStr;
use ::std::marker::PhantomData;
use ::cpp_utils::ConstPtr;

/// Rust alternative to Qt's `QFlags` types.
///
/// `Flags<E>` is an OR-combination of integer values of the enum type `E`.
#[derive(Clone, Copy)]
pub struct QFlags<E> {
    value: c_int,
    _phantom_data: PhantomData<E>,
}

impl<E> From<c_int> for QFlags<E> {
    fn from(value: c_int) -> Self {
        Self {
            value,
            _phantom_data: PhantomData,
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

impl<E, T: Into<QFlags<E>>> ::std::ops::BitOr<T> for QFlags<E> {
    type Output = QFlags<E>;
    fn bitor(self, rhs: T) -> QFlags<E> {
        Self {
            value: self.value | rhs.into().value,
            _phantom_data: PhantomData,
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
            _phantom_data: PhantomData,
        }
    }
}

impl<T> ::std::fmt::Debug for QFlags<T> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
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
    pub unsafe fn connect<R>(&self, receiver: R) -> crate::q_meta_object::Connection
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

mod impl_arguments_compatible;

/// A struct providing valid `argc` and `argv` values for Qt application
/// objects.
///
/// Constructors of `qt_core::core_application::CoreApplication`,
/// `qt_gui::gui_application::GuiApplication` and `qt_widgets::application::Application`
/// require `argc` and `argv` values that are available in C++'s `main` function but
/// not available in Rust. More importantly, `argc` and `argv` must be valid for the entire
/// life of the application. This struct stores list of arguments in a format compatible with
/// `argc` and `argv`, and can be used to initialize Qt application objects.
/// `CoreApplicationArgs` must live longer than the application object.
///
/// `CoreApplication::create_and_exit` convenience function
/// and similar functions in the other application types
/// can be used instead of `CoreApplicationArgs`.
pub struct QCoreApplicationArgs {
    _values: Vec<Vec<u8>>,
    argc: Box<::std::os::raw::c_int>,
    argv: Vec<*mut ::std::os::raw::c_char>,
}

impl QCoreApplicationArgs {
    /// Creates an object containing `args`.
    pub fn new(mut args: Vec<Vec<u8>>) -> QCoreApplicationArgs {
        for arg in &mut args {
            if !arg.ends_with(&[0]) {
                arg.push(0);
            }
        }
        QCoreApplicationArgs {
            argc: Box::new(args.len() as ::std::os::raw::c_int),
            argv: args
                .iter_mut()
                .map(|x| x.as_mut_ptr() as *mut ::std::os::raw::c_char)
                .collect(),
            _values: args,
        }
    }
    /// Creates an object containing empty list of arguments.
    /// Although this is the cheapest way to construct a `CoreApplicationArgs`
    /// object, it's not clear whether Qt considers empty arguments list valid.
    pub fn empty() -> QCoreApplicationArgs {
        QCoreApplicationArgs::from(Vec::new())
    }

    /// Returns `(argc, argv)` values in the form accepted by the application objects'
    /// constructors.
    pub fn get(&mut self) -> (&mut ::std::os::raw::c_int, *mut *mut ::std::os::raw::c_char) {
        (self.argc.as_mut(), self.argv.as_mut_ptr())
    }

    #[cfg(unix)]
    /// Creates an object representing real arguments of the application.
    /// On Windows, this function uses empty argument list for performance reasons because
    /// Qt doesn't use `argc` and `argv` on Windows at all.
    pub fn from_real() -> QCoreApplicationArgs {
        use ::std::os::unix::ffi::OsStringExt;
        let args = ::std::env::args_os().map(|arg| arg.into_vec()).collect();
        QCoreApplicationArgs::from(args)
    }
    #[cfg(windows)]
    /// Creates an object representing real arguments of the application.
    /// On Windows, this function uses empty argument list for performance reasons because
    /// Qt doesn't use `argc` and `argv` on Windows at all.
    pub fn from_real() -> QCoreApplicationArgs {
        // Qt doesn't use argc and argv on Windows anyway
        // TODO: check this
        QCoreApplicationArgs::empty()
    }
}

impl QCoreApplication {
    /// A convenience function for performing proper initialization and de-initialization of
    /// a Qt application.
    ///
    /// This function creates `CoreApplication` with valid `argc` and `argv`, calls the passed
    /// closure `f(app)` with the application object and exist the process with the exit code
    /// returned by the closure. The closure should perform the initialization of the application
    /// and either return immediately or call `CoreApplication::exec()` and return its return value:
    /// ```
    /// fn main() {
    ///   CoreApplication::create_and_exit(|app| {
    ///     // initialization goes here
    ///     CoreApplication::exec()
    ///   })
    /// }
    /// ```
    pub fn create_and_exit<F: FnOnce(&mut QCoreApplication) -> i32>(f: F) -> ! {
        let exit_code = {
            let mut args = QCoreApplicationArgs::from_real();
            let mut app = unsafe { QCoreApplication::new(args.get()) };
            f(app.as_mut())
        }; // drop `app` and `args`
        ::std::process::exit(exit_code)
    }
}

// TODO: split to multiple files
