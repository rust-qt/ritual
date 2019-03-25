use crate::QCoreApplication;
use std::os::raw::{c_char, c_int};
use std::{env, process};

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
    argc: Box<c_int>,
    argv: Vec<*mut c_char>,
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
            argc: Box::new(args.len() as c_int),
            argv: args
                .iter_mut()
                .map(|x| x.as_mut_ptr() as *mut c_char)
                .collect(),
            _values: args,
        }
    }
    /// Creates an object containing empty list of arguments.
    /// Although this is the cheapest way to construct a `CoreApplicationArgs`
    /// object, it's not clear whether Qt considers empty arguments list valid.
    pub fn empty() -> QCoreApplicationArgs {
        QCoreApplicationArgs::new(Vec::new())
    }

    /// Returns `(argc, argv)` values in the form accepted by the application objects'
    /// constructors.
    pub fn get(&mut self) -> (*mut c_int, *mut *mut c_char) {
        let argc = self.argc.as_mut();
        let argv = self.argv.as_mut_ptr();
        (argc, argv)
    }

    #[cfg(unix)]
    /// Creates an object representing real arguments of the application.
    /// On Windows, this function uses empty argument list for performance reasons because
    /// Qt doesn't use `argc` and `argv` on Windows at all.
    pub fn from_real() -> QCoreApplicationArgs {
        use std::os::unix::ffi::OsStringExt;
        let args = env::args_os().map(|arg| arg.into_vec()).collect();
        QCoreApplicationArgs::new(args)
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
    /// ```no_run
    /// use moqt_core::QCoreApplication;
    ///
    /// fn main() {
    ///     QCoreApplication::create_and_exit(|app| {
    ///         unsafe {
    ///             // initialization goes here
    ///             QCoreApplication::exec()
    ///         }
    ///     })
    /// }
    /// ```
    pub fn create_and_exit<F: FnOnce(::cpp_utils::Ptr<QCoreApplication>) -> i32>(f: F) -> ! {
        let exit_code = {
            let mut args = QCoreApplicationArgs::from_real();
            let (argc, argv) = args.get();
            unsafe {
                let mut app = QCoreApplication::new2(
                    ::cpp_utils::Ptr::new(argc),
                    ::cpp_utils::Ptr::new(argv),
                );
                f(app.as_mut_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }
}
