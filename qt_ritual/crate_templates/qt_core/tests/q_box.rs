use qt_core::{QBox, QObject};

#[test]
fn qbox1() {
    unsafe {
        let mut obj = QBox::new(QObject::new_0a().into_ptr());
        assert_eq!(obj.children().length(), 0);
        {
            let _obj2 = QBox::new(QObject::new_1a(&mut obj).into_ptr());
            assert_eq!(obj.children().length(), 1);
        }
        assert_eq!(obj.children().length(), 1);
    }
}

#[test]
fn qbox2() {
    unsafe {
        let mut obj = QBox::new(QObject::new_0a().into_ptr());
        let obj2 = QBox::new(QObject::new_1a(&mut obj).into_ptr());
        assert!(!obj2.is_null());
        drop(obj);
        assert!(obj2.is_null());
    }
}

#[test]
fn qbox3() {
    unsafe {
        let mut obj = QBox::new(QObject::new_0a().into_ptr());
        let obj2 = QBox::new(QObject::new_1a(&mut obj).into_ptr());
        assert!(!obj2.is_null());
        let _obj1 = obj.into_q_ptr();
        assert!(!obj2.is_null());
    }
}
