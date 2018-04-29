extern crate qt_core;
use qt_core::core_application::CoreApplication;
use qt_core::timer::Timer;
use qt_core::connection::Signal;

use qt_core::libc::c_void;
use qt_core::slots::raw::RawSlotNoArgs;
extern "C" fn func1(data: *mut c_void) {
  let data: usize = unsafe { std::mem::transmute(data) };
  println!("about_to_quit: {}", data);
}

#[test]
fn timer_quit() {
  println!("timer_quit: Started");
  CoreApplication::create_and_exit(|app| {
    let mut slot1 = RawSlotNoArgs::new();
    unsafe {
      slot1.set(func1, std::mem::transmute(42usize));
    }
    app.signals().about_to_quit().connect(slot1.as_ref());

    let mut timer = Timer::new();
    timer.signals().timeout().connect(&app.slots().quit());
    timer.start(1000);
    CoreApplication::exec()
  })
}
