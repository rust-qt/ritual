use qt_widgets::{
    cpp_utils::{CppBox, Ptr, StaticUpcast},
    qt_core::QString,
    qt_core::Slot,
    QApplication, QLineEdit, QMessageBox, QPushButton, QVBoxLayout, QWidget,
};

struct Form<'a> {
    _widget: CppBox<QWidget>,
    _button: Ptr<QPushButton>,
    _line_edit: Ptr<QLineEdit>,
    button_clicked: Slot<'a>,
    line_edit_edited: Slot<'a>,
}

impl<'a> Form<'a> {
    fn new() -> Form<'a> {
        unsafe {
            let mut widget = QWidget::new_0a();
            let mut layout = QVBoxLayout::new_1a(widget.as_mut_ptr());
            let mut line_edit = QLineEdit::new3();

            layout.add_widget(line_edit.static_upcast_mut().into());

            let line_edit = line_edit.into_ptr();
            let mut button = QPushButton::new5(QString::from_std_str("Start").as_ref());
            button.set_enabled(false);

            layout.add_widget(button.static_upcast_mut().static_upcast_mut().into());

            let button = button.into_ptr();
            layout.into_ptr();
            widget.show();

            let mut button1 = button;
            let line_edit1 = line_edit;
            let widget1 = widget.as_mut_ptr();

            let form = Form {
                _widget: widget,
                _button: button,
                _line_edit: line_edit,
                button_clicked: Slot::new(move || {
                    let text = line_edit1.text();
                    QMessageBox::information6(
                        widget1,
                        QString::from_std_str("My title").as_ref(),
                        QString::from_std_str("Text: \"%1\". Congratulations!")
                            .arg56(text.as_ref())
                            .as_ref(),
                    );
                }),
                line_edit_edited: Slot::new(move || {
                    button1.set_enabled(!line_edit1.text().is_empty());
                }),
            };
            button.clicked().connect(&form.button_clicked);
            line_edit.text_edited().connect(&form.line_edit_edited);
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
