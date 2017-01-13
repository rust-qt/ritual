extern crate qt_core;
use qt_core::core_application::CoreApplication;
use qt_core::timer::Timer;
use qt_core::cpp_utils::*;
use qt_core::connections::Signal;

use qt_core::libc::c_void;
use qt_core::slots::ExternSlotNoArgs;
extern "C" fn func1(data: *mut c_void) {
  let data: usize = unsafe { std::mem::transmute(data) };
  println!("about_to_quit: {}", data);
}

#[test]
fn timer_quit() {
  println!("timer_quit: Started");
  CoreApplication::create_and_exit(|app| {
    let mut slot1 = ExternSlotNoArgs::new();
    slot1.set(func1, unsafe { std::mem::transmute(42usize) });
    app.signals().about_to_quit().connect(slot1.as_ref());

    let mut timer = Timer::new(AsBox);
    timer.signals().timeout().connect(&app.slots().quit());
    timer.start(1000);
    CoreApplication::exec()
  })
}
