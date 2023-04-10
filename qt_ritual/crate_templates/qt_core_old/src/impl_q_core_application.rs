use crate::{QCoreApplication, QString};
use std::iter::once;
use std::os::raw::{c_char, c_int};
use std::process;

/// A struct providing valid `argc` and `argv` values for Qt application
/// objects.
///
/// `QCoreApplication` shouldn't be used directly. Instead, use `init` function on
/// the application types (`qt_core::QCoreApplication`,
/// `qt_gui::QGuiApplication`, and `qt_widgets::QApplication`).
///
/// Constructors of Qt application classes
/// require `argc` and `argv` values that are available in C++'s `main` function but
/// not available in Rust. Qt also expects `argc` and `argv` to be encoded in a particular
/// (local 8-bit) encoding. `argc` and `argv` must be valid for the entire
/// life of the application. This struct stores list of arguments in a format compatible with
/// `argc` and `argv`, and can be used to initialize Qt application objects.
///
/// `QCoreApplicationArgs` must live longer than the application object.
pub struct QCoreApplicationArgs {
    _values: Vec<Vec<u8>>,
    argc: Box<c_int>,
    argv: Box<[*mut c_char]>,
}

impl QCoreApplicationArgs {
    /// Creates an object containing `args`.
    pub fn new() -> QCoreApplicationArgs {
        // Qt uses `QString::fromLocal8Bit()` to decode `argv`,
        // so we use `QString::toLocal8Bit()` to encode it.
        let mut args = std::env::args()
            .map(|arg| unsafe {
                QString::from_std_str(&arg)
                    .to_local8_bit()
                    .as_slice()
                    .iter()
                    .map(|&c| c as u8)
                    .chain(once(0))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        QCoreApplicationArgs {
            argc: Box::new(args.len() as c_int),
            argv: args
                .iter_mut()
                .map(|x| x.as_mut_ptr() as *mut c_char)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            _values: args,
        }
    }

    /// Returns `(argc, argv)` values in the form accepted by the application objects'
    /// constructors.
    pub fn get(&mut self) -> (*mut c_int, *mut *mut c_char) {
        (self.argc.as_mut(), self.argv.as_mut_ptr())
    }
}

impl QCoreApplication {
    /// A convenience function for performing proper initialization and de-initialization of
    /// a Qt application.
    ///
    /// This function creates a `QCoreApplication` object with valid `argc` and `argv`,
    /// calls the passed closure `f(app)` with the application object
    /// and exits the process with the exit code returned by the closure.
    /// The closure should perform the initialization of the application
    /// and either return immediately or call `QCoreApplication::exec()`
    /// and return its return value:
    /// ```no_run
    /// use qt_core::QCoreApplication;
    ///
    /// fn main() {
    ///     QCoreApplication::init(|app| {
    ///         unsafe {
    ///             // initialization goes here
    ///             QCoreApplication::exec()
    ///         }
    ///     })
    /// }
    /// ```
    pub fn init<F: FnOnce(::cpp_core::Ptr<QCoreApplication>) -> i32>(f: F) -> ! {
        let exit_code = {
            unsafe {
                let mut args = QCoreApplicationArgs::new();
                let (argc, argv) = args.get();
                let app = QCoreApplication::new_2a(argc, argv);
                f(app.as_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }
}
