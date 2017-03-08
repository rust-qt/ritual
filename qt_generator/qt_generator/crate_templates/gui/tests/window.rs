extern crate qt_gui;

use qt_gui::gui_application::GuiApplication;
use qt_gui::window::Window;
use qt_gui::qt_core::point::Point;
use qt_gui::cpp_utils::*;

#[test]
fn window1() {
  let _app = GuiApplication::new((&mut 0i32, &mut (&mut 0i8 as *mut i8) as *mut *mut i8));

  let mut a = Window::new();
  let mut b = Window::new(a.as_mut_ptr());
  let mut c = Window::new(b.as_mut_ptr());
  a.set_geometry((10, 10, 300, 300));
  b.set_geometry((20, 20, 200, 200));
  c.set_geometry((40, 40, 100, 100));

  let point = Point::new((100, 100));
  let r1 = a.map_to_global((&point));
  assert_eq!(r1.x(), 110);
  assert_eq!(r1.y(), 110);

}
