use crate::QApplication;
use cpp_core::{MutPtr, MutRef};
use qt_core::QCoreApplicationArgs;
use std::process;

impl QApplication {
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
    ///     QApplication::init(|app| {
    ///         unsafe {
    ///             // initialization goes here
    ///             QApplication::exec()
    ///         }
    ///     })
    /// }
    /// ```
    pub fn init<F: FnOnce(::cpp_core::MutPtr<QApplication>) -> i32>(f: F) -> ! {
        let exit_code = {
            unsafe {
                let mut args = QCoreApplicationArgs::new();
                let (argc, argv) = args.get();
                let mut app =
                    QApplication::new_2a(MutRef::from_raw(argc).unwrap(), MutPtr::from_raw(argv));
                f(app.as_mut_ptr())
            }
        }; // drop `app` and `args`
        process::exit(exit_code)
    }
}
