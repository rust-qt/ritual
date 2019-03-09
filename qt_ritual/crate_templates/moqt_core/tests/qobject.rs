use moqt_core::QObject;
use std::ffi::CStr;

#[test]
fn qobject() {
    unsafe {
        let obj1 = QObject::new2();
        let obj2 = QObject::new2();
        obj1.destroyed().connect(obj2.delete_later());

        let args = QObject::next_connect_args();
        assert_eq!(args.sender().as_ptr(), obj1.as_ptr().as_ptr());
        assert_eq!(args.receiver().as_ptr(), obj2.as_ptr().as_ptr());

        let signal = CStr::from_ptr(args.signal().as_ptr()).to_str().unwrap();
        assert_eq!(signal, "2destroyed(QObject*)");

        let method = CStr::from_ptr(args.method().as_ptr()).to_str().unwrap();
        assert_eq!(method, "1deleteLater()");
    }
}
