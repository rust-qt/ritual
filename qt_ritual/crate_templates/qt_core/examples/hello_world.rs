use qt_core::{q_debug, QCoreApplication, QString};

fn main() {
    QCoreApplication::init(|_app| unsafe {
        let _ = q_debug!() << QString::from_std_str("Hello World!").as_ref();
        0
    })
}
