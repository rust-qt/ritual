use cpp_core::{NullPtr, Ref};
use moqt_core::{QCoreApplication, QString, SlotOfQString};

#[test]
fn q_core_application() {
    QCoreApplication::init(|app| unsafe {
        let slot = SlotOfQString::new(NullPtr, |_name: Ref<QString>| ());
        app.app_name_changed().connect(&slot);
        0
    });
}
