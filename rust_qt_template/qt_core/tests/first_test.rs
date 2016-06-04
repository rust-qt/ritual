extern crate qt_core;
use qt_core::ffi;

#[test]
fn it_works() {
  assert!(true);
}

#[test]
fn test1() {
  unsafe {
    //ffi::QDebug_destructor(0 as *mut qt_core::debug::QDebug);
    let x = ffi::QRect_new_left_top_width_height(1, 2, 3, 4);
    assert!(ffi::QRect_width(x) == 3);
    ffi::QRect_delete(x);
  }
}

#[test]
fn test2() {
  assert!(qt_core::q_rect::QRect::test1() == 42);
}
