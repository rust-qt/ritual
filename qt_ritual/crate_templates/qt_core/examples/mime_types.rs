use qt_core::{q_debug, QCoreApplication};

fn main() {
    QCoreApplication::init(|app| unsafe {
        q_debug!() << QCoreApplication::arguments().as_ref();
        0
    })
}
