use moqt_gui::QWindow;
use moqt_core::BasicClass;
use cpp_utils::{CppBox, Ptr};

#[test]
fn create() {
    unsafe {
        let mut window = QWindow::new();
        let mut object: CppBox<BasicClass> = window.get_basic_class();
        assert_eq!(object.foo(), 42);
        let mut object_ptr: Ptr<BasicClass> = window.get_basic_class_ptr();
        assert_eq!(object_ptr.foo(), 43);
    }
}
