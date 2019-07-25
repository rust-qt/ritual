use qt_core::QCoreApplication;
use qt_core::QMessageLogger;
use qt_core::QString;

fn main() {
    QCoreApplication::init(|_app| unsafe {
        qt_core::q_set_message_pattern(&QString::from_std_str(
            "file='%{file}', line='%{line}', function='%{function}': %{message}",
        ));

        let _ =
            &QMessageLogger::new_0a().debug_0a() << QString::from_std_str("Hello World!").as_ref();
        0
    })
}
