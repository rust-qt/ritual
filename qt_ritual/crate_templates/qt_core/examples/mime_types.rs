use qt_core::{q_debug, qdebug, QCoreApplication, QListOfQString};

fn main() {
    QCoreApplication::init(|_app| unsafe {
        let arguments = QCoreApplication::arguments();
        let arguments = arguments.static_upcast::<QListOfQString>();
        println!("arguments: {:?}", qdebug(arguments));

        for arg in arguments {
            println!("argument: {:?}", qdebug(arg));
        }
        0
    })
}
