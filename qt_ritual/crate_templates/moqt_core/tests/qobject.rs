use moqt_core::QObject;
use cpp_utils::CppDeletable;

#[test]
fn qobject() {
    unsafe {
        let mut obj = QObject::new2();
        obj.delete_later();
    }
}
