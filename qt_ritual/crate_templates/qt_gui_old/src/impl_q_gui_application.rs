use crate::QGuiApplication;
use qt_core::QCoreApplicationArgs;
use std::process;

impl QGuiApplication {
    /// A convenience function for performing proper initialization and de-initialization of
    /// a Qt application.
    ///
    /// This function creates a `QGuiApplication` object with valid `argc` and `argv`,
    /// calls the passed closure `f(app)` with the application object
    /// and exits the process with the exit code returned by the closure.
    /// The closure should perform the initialization of the application
    /// and either return immediately or call `QGuiApplication::exec()`
    /// and return its return value:
    /// ```no_run
    /// use qt_gui::QGuiApplication;
    ///
    /// fn main() {
    ///     QGuiApplication::init(|app| {
    ///         unsafe {
    ///             // initialization goes here
    ///             QGuiApplication::exec()
    ///         }
    ///     })
    /// }
    /// ```
    pub fn init<F: FnOnce(::cpp_core::Ptr<QGuiApplication>) -> i32>(f: F) -> ! {
        let exit_code = {
            unsafe {
                let mut args = QCoreApplicationArgs::new();
                let (argc, argv) = args.get();
                let app = QGuiApplication::new_2a(argc, argv);
                f(app.as_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }
}
