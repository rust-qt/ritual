extern crate qt_core;
use qt_core::ffi;

#[test]
fn it_works() {
    assert!(true);
}

#[test]
fn test1() {
  unsafe {
    let x = ffi::QRect_new_x_y_width_height(1, 2, 3, 4);
    assert!(ffi::QRect_width(x) == 3);
    ffi::QRect_delete(x);
  }
}
