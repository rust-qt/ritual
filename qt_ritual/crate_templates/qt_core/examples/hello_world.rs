use qt_core::QCoreApplication;
use qt_core::QMessageLogger;
use qt_core::QString;

fn main() {
    QCoreApplication::create_and_exit(|_app| unsafe {
        QMessageLogger::new_0a()
            .debug_0a()
            .operator_shl15(QString::from_std_str("Hello World!").as_ref());
        QCoreApplication::exec()
    })
}
