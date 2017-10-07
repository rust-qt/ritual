extern crate qt_gui;

use qt_gui::gui_application::GuiApplication;
use qt_gui::window::Window;
use qt_gui::qt_core::point::Point;

#[test]
fn window1() {
  GuiApplication::create_and_exit(|_| {
    let mut a = Window::new();
    let mut b = unsafe { Window::new_unsafe(a.as_mut_ptr()) };
    let mut c = unsafe { Window::new_unsafe(b.as_mut_ptr()) };
    a.set_geometry((10, 10, 300, 300));
    b.set_geometry((20, 20, 200, 200));
    c.set_geometry((40, 40, 100, 100));

    let point = Point::new((100, 100));
    let r1 = a.map_to_global((&point));
    assert_eq!(r1.x(), 110);
    assert_eq!(r1.y(), 110);
    0
  })
}
