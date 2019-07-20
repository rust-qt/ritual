use cpp_utils::Ptr;
use qt_core::{QCoreApplication, QTimer, RawSlot};
use std::ffi::c_void;

extern "C" fn func1(data: *mut c_void) {
    println!("about_to_quit: {}", data as usize);
}

#[test]
fn timer_quit() {
    println!("timer_quit: Started");
    QCoreApplication::init(|app| unsafe {
        let mut slot1 = RawSlot::new();
        slot1.set(Some(func1), Ptr::from_raw(42 as *mut c_void));
        app.about_to_quit().connect(&slot1);

        let mut timer = QTimer::new_0a();
        timer.timeout().connect(app.slot_quit());
        timer.start_1a(1000);
        QCoreApplication::exec()
    })
}
