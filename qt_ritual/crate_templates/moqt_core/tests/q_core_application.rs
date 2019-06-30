use cpp_utils::ConstRef;
use moqt_core::{QCoreApplication, QString, SlotOfQString};

#[test]
fn q_core_application() {
    unsafe {
        let obj1 = QCoreApplication::new();
        let slot = SlotOfQString::new(|_name: ConstRef<QString>| ());
        obj1.app_name_changed().connect(&slot);
    }
}
