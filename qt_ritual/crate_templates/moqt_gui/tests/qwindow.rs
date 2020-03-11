use cpp_core::{CppBox, Ptr};
use moqt_core::{BasicClass, QBox, QPoint, QPtr, QVectorOfInt};
use moqt_gui::{get_window, QVectorOfQWindow, QWindow};

#[test]
fn test_qwindow() {
    unsafe {
        let window = QWindow::new();
        let object: CppBox<BasicClass> = window.get_basic_class();
        assert_eq!(object.foo(), 42);
        let object_ptr: Ptr<BasicClass> = window.get_basic_class_ptr();
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
        let window: QPtr<QWindow> = get_window();
        assert!(window.is_null());
    }
}

#[test]
fn test_with_vectors() {
    unsafe {
        let window: QBox<QWindow> = QWindow::new();

        let vec = QVectorOfInt::new();
        vec.push(&10);
        vec.push(&12);
        vec.push(&14);
        vec.push(&16);
        let r = window.show_vector_of_int(vec.as_ref());
        assert_eq!(r, 4);

        let vec2 = QVectorOfQWindow::new();
        vec2.push(&get_window().as_mut_raw_ptr());
        vec2.push(&get_window().as_mut_raw_ptr());
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

#[test]
fn template_classes() {
    unsafe {
        let _a: CppBox<moqt_core::ns1::Templated1OfInt> = moqt_gui::get_same_template1();
        let _b: CppBox<moqt_core::ns1::class_ns::Templated2OfBool> = moqt_gui::get_same_template2();
        let _c: CppBox<moqt_core::Templated3OfInt> = moqt_gui::get_same_template3();

        let d: CppBox<moqt_gui::Templated1OfFloat> = moqt_gui::get_new_template1();
        d.x();
        let e: CppBox<moqt_gui::Templated2OfFloat> = moqt_gui::get_new_template2();
        e.y();
        let f: CppBox<moqt_gui::Templated3OfFloat> = moqt_gui::get_new_template3();
        f.get();
    }
}
