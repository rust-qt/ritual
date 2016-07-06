extern crate qt_core;

#[test]
fn it_works() {
  assert!(true);
}

#[test]
fn test1() {
  let r = qt_core::rect::Rect::new_args_left_top_width_height(1, 2, 3, 4);
  assert!(r.width() == 3);
}
