use cpp_utils::Ptr;
use qt_core::{QCoreApplication, QTimer, RawSlot};
use std::ffi::c_void;

extern "C" fn func1(data: *mut c_void) {
    println!("about_to_quit: {}", data as usize);
}

#[test]
fn timer_quit() {
    println!("timer_quit: Started");
    QCoreApplication::create_and_exit(|app| unsafe {
        let mut slot1 = RawSlot::new();
        slot1.set(Some(func1), Ptr::new(42 as *mut c_void));
        app.about_to_quit().connect(&slot1);

        let mut timer = QTimer::new2();
        timer.timeout().connect(app.quit());
        timer.start3(1000);
        QCoreApplication::exec()
    })
}
