use qt_core::{q_debug, QCoreApplication, QListOfQString};

fn main() {
    QCoreApplication::init(|_app| unsafe {
        let arguments = QCoreApplication::arguments();
        let _ = q_debug!() << arguments.static_upcast::<QListOfQString>();
        0
    })
}
