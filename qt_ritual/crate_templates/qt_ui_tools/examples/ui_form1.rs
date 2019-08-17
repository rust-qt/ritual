use qt_ui_tools::{
    cpp_utils::CppBox,
    qt_core::{QBuffer, QByteArray, Slot},
    qt_widgets::{QApplication, QWidget},
    QUiLoader,
};

struct Form<'a> {
    _widget: CppBox<QWidget>,
    _check_box_toggled: Slot<'a>,
}

impl<'a> Form<'a> {
    fn new() -> Form<'a> {
        unsafe {
            let form_data =
                include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/form1.ui"));
            let mut byte_array = QByteArray::from_slice(form_data);
            let mut buffer = QBuffer::from_q_byte_array(&mut byte_array);
            let mut ui_loader = QUiLoader::new_0a();
            let mut widget = CppBox::new(ui_loader.load_1a(&mut buffer)).expect("load failed");
            widget.show();

            let form = Form {
                _widget: widget,
                _check_box_toggled: Slot::new(move || {}),
            };
            form
        }
    }
}

fn main() {
    QApplication::init(|_| unsafe {
        let _form = Form::new();
        QApplication::exec()
    })
}
