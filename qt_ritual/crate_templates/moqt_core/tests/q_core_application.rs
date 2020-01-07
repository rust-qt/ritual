use cpp_core::Ref;
use moqt_core::{QCoreApplication, QString, SlotOfQString};

#[test]
fn q_core_application() {
    QCoreApplication::init(|app| unsafe {
        let slot = SlotOfQString::with(|_name: Ref<QString>| ());
        app.app_name_changed().connect(&slot);
        0
    });
}
