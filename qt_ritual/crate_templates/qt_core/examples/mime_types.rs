use qt_core::{q_debug, QCoreApplication, QListOfQString};

fn main() {
    QCoreApplication::init(|_app| unsafe {
        let arguments = QCoreApplication::arguments();
        let arguments = arguments.static_upcast::<QListOfQString>();
        let _ = q_debug!() << arguments;

        for arg in arguments {
            let _ = q_debug!() << arg;
        }
        0
    })
}
