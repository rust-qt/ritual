use qt_core::QCoreApplication;
use qt_core::QMessageLogger;
use qt_core::QString;

fn main() {
    QCoreApplication::create_and_exit(|_app| unsafe {
        let _ =
            &QMessageLogger::new_0a().debug_0a() << QString::from_std_str("Hello World!").as_ref();
        QCoreApplication::exec()
    })
}
