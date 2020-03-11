use cpp_core::NullPtr;
use moqt_core::{QObject, QPtr, SlotOfInt};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn qobject() {
    unsafe {
        let obj1 = QObject::new_0a();
        let obj2 = QObject::new_0a();
        let c = obj1.destroyed().connect(obj2.slot_delete_later());
        assert!(c.is_valid());

        let args = QObject::next_connect_args();
        assert_eq!(args.sender().as_raw_ptr(), obj1.as_raw_ptr());
        assert_eq!(args.receiver().as_raw_ptr(), obj2.as_raw_ptr());

        let signal = args.signal().to_c_str().to_str().unwrap();
        assert_eq!(signal, "2destroyed(QObject *)");

        let method = args.method().to_c_str().to_str().unwrap();
        assert_eq!(method, "1deleteLater()");
    }
}

#[test]
fn closure_slot_connect() {
    unsafe {
        let obj1 = QObject::new_0a();
        let counter = Rc::new(RefCell::new(0));
        let counter_handle = Rc::clone(&counter);
        let slot = SlotOfInt::new(NullPtr, move |arg| {
            *counter_handle.borrow_mut() += arg;
        });
        let c = obj1.object_name_changed().connect(&slot);
        assert!(c.is_valid());

        let args = QObject::next_connect_args();
        assert_eq!(args.sender().as_raw_ptr(), obj1.as_raw_ptr());

        let signal = args.signal().to_c_str().to_str().unwrap();
        assert_eq!(signal, "2objectNameChanged(int)");

        let slot_as_qobject: QPtr<QObject> = slot.static_upcast();
        assert_eq!(args.receiver().as_raw_ptr(), slot_as_qobject.as_raw_ptr());

        let method = args.method().to_c_str().to_str().unwrap();
        assert_eq!(method, "1slot_(int)");

        assert_eq!(*counter.borrow(), 0);
        slot.slot(2);
        assert_eq!(*counter.borrow(), 2);
        slot.slot(4);
        assert_eq!(*counter.borrow(), 6);

        slot.set(|_| ());
        slot.slot(8);
        assert_eq!(*counter.borrow(), 6);
    }
}
