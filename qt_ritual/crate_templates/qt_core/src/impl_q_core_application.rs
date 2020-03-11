use crate::{QCoreApplication, QString};
use cpp_core::{Ptr, Ref};
use std::iter::once;
use std::os::raw::{c_char, c_int};
use std::process;

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
/// `CoreApplication::init` convenience function
/// and similar functions in the other application types
/// can be used instead of `CoreApplicationArgs`.
pub struct QCoreApplicationArgs {
    _values: Vec<Vec<u8>>,
    argc: Box<c_int>,
    argv: Vec<*mut c_char>,
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
                .collect(),
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
    /// This function creates `CoreApplication` with valid `argc` and `argv`, calls the passed
    /// closure `f(app)` with the application object and exist the process with the exit code
    /// returned by the closure. The closure should perform the initialization of the application
    /// and either return immediately or call `CoreApplication::exec()` and return its return value:
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
                let app =
                    QCoreApplication::new_2a(Ref::from_raw(argc).unwrap(), Ptr::from_raw(argv));
                f(app.as_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }
}
