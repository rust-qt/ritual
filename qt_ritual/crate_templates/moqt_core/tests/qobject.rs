use moqt_core::QObject;

#[test]
fn qobject() {
    unsafe {
        let mut obj = QObject::new2();
        obj.delete_later2();
    }
}
