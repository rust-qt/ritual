use cpp_utils::{CppBox, MutPtr};
use moqt_core::{BasicClass, QPoint, QVectorOfInt};
use moqt_gui::{get_window, QVectorOfQWindow, QWindow};

#[test]
fn test_qwindow() {
    unsafe {
        let mut window = QWindow::new();
        let mut object: CppBox<BasicClass> = window.get_basic_class();
        assert_eq!(object.foo(), 42);
        let mut object_ptr: MutPtr<BasicClass> = window.get_basic_class_ptr();
        assert_eq!(object_ptr.foo(), 43);

        let point: CppBox<QPoint> = window.pos();
        assert_eq!(point.x(), 0);
        assert_eq!(point.y(), 0);
        window.set_pos(QPoint::new_2a(2, -3).as_ref());
        let point: CppBox<QPoint> = window.pos();
        assert_eq!(point.x(), 55);
        assert_eq!(point.y(), -3);
    }
}

#[test]
fn test_get_window() {
    unsafe {
        let window: MutPtr<QWindow> = get_window();
        assert!(window.is_null());
    }
}

#[test]
fn test_with_vectors() {
    unsafe {
        let mut window: CppBox<QWindow> = QWindow::new();

        let mut vec = QVectorOfInt::new();
        vec.push(10);
        vec.push(12);
        vec.push(14);
        vec.push(16);
        let r = window.show_vector_of_int(vec.as_ref());
        assert_eq!(r, 4);

        let mut vec2 = QVectorOfQWindow::new();
        vec2.push(get_window());
        vec2.push(get_window());
        let r = window.show_vector_of_windows(vec2.as_ref());
        assert_eq!(r, 2);
    }
}

#[test]
fn reexport() {
    use moqt_gui::moqt_core::QPoint;
    unsafe {
        let _ = QPoint::new_0a();
    }
}
