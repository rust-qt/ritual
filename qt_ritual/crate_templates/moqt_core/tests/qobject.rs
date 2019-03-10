use moqt_core::{QObject, RawSlotOfCInt};
use cpp_utils::{Ptr, StaticUpcast};
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::os::raw::{c_void, c_int};

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

#[test]
fn raw_slot() {
    unsafe {
        static FLAG: AtomicBool = AtomicBool::new(false);
        extern "C" fn hook(data: *mut c_void, value: c_int) {
            assert_eq!(value, 7);
            let old = FLAG.swap(true, Ordering::SeqCst);
            assert!(!old);
        }

        let mut obj = RawSlotOfCInt::new();
        obj.set(hook, Ptr::new(5 as *mut c_void));
        assert!(!FLAG.load(Ordering::SeqCst));
        obj.custom_slot(7);
        assert!(FLAG.load(Ordering::SeqCst));
    }
}

#[test]
fn raw_slot_connect() {
    unsafe {
        let obj1 = QObject::new2();
        let mut slot = RawSlotOfCInt::new();
        obj1.object_name_changed().connect(&slot);

        let args = QObject::next_connect_args();
        assert_eq!(args.sender().as_ptr(), obj1.as_ptr().as_ptr());
        let slot_as_qobject: Ptr<QObject> = slot.static_upcast_mut();
        assert_eq!(args.receiver().as_ptr(), slot_as_qobject.as_ptr());

        let signal = CStr::from_ptr(args.signal().as_ptr()).to_str().unwrap();
        assert_eq!(signal, "2objectNameChanged(int)");

        let method = CStr::from_ptr(args.method().as_ptr()).to_str().unwrap();
        assert_eq!(method, "1custom_slot(int)");
    }
}
