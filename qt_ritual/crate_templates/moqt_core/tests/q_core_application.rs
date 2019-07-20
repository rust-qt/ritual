use cpp_utils::ConstRef;
use moqt_core::{QCoreApplication, QString, SlotOfQString};

#[test]
fn q_core_application() {
    QCoreApplication::init(|app| unsafe {
        let slot = SlotOfQString::new(|_name: ConstRef<QString>| ());
        app.app_name_changed().connect(&slot);
        0
    });
}
