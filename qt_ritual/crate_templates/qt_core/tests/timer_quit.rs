use qt_core::QCoreApplication;
use qt_core::QTimer;
use qt_core::RawSlot;
use qt_core::Signal;
use std::ffi::c_void;

extern "C" fn func1(data: *mut c_void) {
    let data: usize = unsafe { std::mem::transmute(data) };
    println!("about_to_quit: {}", data);
}

#[test]
fn timer_quit() {
    println!("timer_quit: Started");
    QCoreApplication::create_and_exit(|app| {
        let mut slot1 = RawSlot::new();
        unsafe {
            slot1.set(func1, 42 as *mut c_void);
        }
        app.about_to_quit().connect(&slot1);

        let mut timer = QTimer::new();
        timer.timeout().connect(&app.quit());
        timer.start(1000);
        QCoreApplication::exec()
    })
}
