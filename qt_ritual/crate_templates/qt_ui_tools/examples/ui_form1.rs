use qt_ui_tools::{
    cpp_utils::{CppBox, StaticUpcast},
    qt_core::{q_io_device::OpenModeFlag, QFile, QString, Slot},
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
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/form1.ui");
            let mut file = QFile::new2(QString::from_std_str(path).as_ref());
            assert!(file.open(OpenModeFlag::ReadOnly.into()));

            let mut ui_loader = QUiLoader::new_0a();
            let mut widget =
                CppBox::new(ui_loader.load_1a(file.static_upcast_mut().static_upcast_mut().into()))
                    .expect("load failed");
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
    QApplication::create_and_exit(|_| unsafe {
        let _form = Form::new();
        QApplication::exec()
    })
}
