use moqt_gui::{QWindow, get_window};
use moqt_core::BasicClass;
use cpp_utils::{CppBox, Ptr};

#[test]
fn test_qwindow() {
    unsafe {
        let mut window = QWindow::new();
        let mut object: CppBox<BasicClass> = window.get_basic_class();
        assert_eq!(object.foo(), 42);
        let mut object_ptr: Ptr<BasicClass> = window.get_basic_class_ptr();
        assert_eq!(object_ptr.foo(), 43);
    }
}

#[test]
fn test_get_window() {
    unsafe {
        let window: Ptr<QWindow> = get_window();
        assert!(window.is_null());
    }
}
