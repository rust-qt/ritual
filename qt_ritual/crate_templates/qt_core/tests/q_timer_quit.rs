use cpp_core::NullPtr;
use qt_core::{QCoreApplication, QTimer, SlotNoArgs};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn timer_quit() {
    QCoreApplication::init(|app| unsafe {
        let value = Rc::new(RefCell::new(Some(42)));
        let value2 = Rc::clone(&value);
        let slot1 = SlotNoArgs::new(NullPtr, move || {
            assert_eq!(value2.borrow_mut().take(), Some(42));
        });
        let c = app.about_to_quit().connect(&slot1);
        assert!(c.is_valid());

        let timer = QTimer::new_0a();
        let c = timer.timeout().connect(app.slot_quit());
        assert!(c.is_valid());
        timer.start_1a(1000);
        let r = QCoreApplication::exec();
        assert!(value.borrow().is_none());
        r
    })
}
