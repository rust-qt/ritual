use crate::QApplication;
use cpp_utils::CppBox;
use qt_core::QCoreApplicationArgs;
use std::process;

impl QApplication {
    pub fn new() -> CppBox<Self> {
        let mut args = QCoreApplicationArgs::from_real();
        let (argc, argv) = args.get();
        unsafe {
            QApplication::new_2a(
                ::cpp_utils::Ref::from_raw(argc).unwrap(),
                ::cpp_utils::Ptr::from_raw(argv),
            )
        }
    }

    /// A convenience function for performing proper initialization and de-initialization of
    /// a Qt application.
    ///
    /// This function creates `CoreApplication` with valid `argc` and `argv`, calls the passed
    /// closure `f(app)` with the application object and exist the process with the exit code
    /// returned by the closure. The closure should perform the initialization of the application
    /// and either return immediately or call `CoreApplication::exec()` and return its return value:
    /// ```no_run
    /// use qt_widgets::QApplication;
    ///
    /// fn main() {
    ///     QApplication::create_and_exit(|app| {
    ///         unsafe {
    ///             // initialization goes here
    ///             QApplication::exec()
    ///         }
    ///     })
    /// }
    /// ```
    pub fn create_and_exit<F: FnOnce(::cpp_utils::Ptr<QApplication>) -> i32>(f: F) -> ! {
        let exit_code = {
            unsafe {
                let mut app = QApplication::new();
                f(app.as_mut_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }
}
