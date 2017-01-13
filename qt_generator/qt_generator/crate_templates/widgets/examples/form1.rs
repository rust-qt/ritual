extern crate qt_widgets;
use qt_widgets::cpp_utils::{CppBox, AsBox, AsStruct, StaticCast};
use qt_widgets::application::Application;
use qt_widgets::qt_core::connections::Signal;

use qt_widgets::widget::Widget;
use qt_widgets::push_button::PushButton;
use qt_widgets::line_edit::LineEdit;
use qt_widgets::qvb_ox_layout::QvbOxLayout as VBoxLayout;
use qt_widgets::qt_core::string::String;
use qt_widgets::qt_core::slots::SlotNoArgs;
use qt_widgets::message_box::MessageBox;

use std::cell::RefCell;

struct Form<'a> {
  widget: CppBox<Widget>,
  button: *mut PushButton,
  line_edit: *mut LineEdit,
  button_clicked: SlotNoArgs<'a>,
  line_edit_edited: SlotNoArgs<'a>,
}

fn uref<T>(ptr: *mut T) -> &'static mut T {
  unsafe { ptr.as_mut() }.expect("null pointer in uref")
}

impl<'a> Form<'a> {
  fn new() -> Form<'a> {
    let mut widget = Widget::new(AsBox);
    let mut layout = VBoxLayout::new((widget.as_mut_ptr(), AsBox));
    let mut line_edit = LineEdit::new(AsBox);
    layout.add_widget(line_edit.static_cast_mut() as *mut _);
    let line_edit = line_edit.into_raw();
    let mut button = PushButton::new((&String::from_std_str("Start"), AsBox));
    button.set_enabled(false);
    layout.add_widget(button.static_cast_mut() as *mut _);
    let button = button.into_raw();
    layout.into_raw();
    widget.show();

    let button1 = button;
    let line_edit1 = line_edit;
    let widget1 = widget.as_mut_ptr();

    let form = Form {
      widget: widget,
      button: button,
      line_edit: line_edit,
      button_clicked: SlotNoArgs::new(move || {
        let text = uref(line_edit1).text(AsStruct);
        MessageBox::information((widget1,
                                 &String::from_std_str("My title"),
                                 &String::from_std_str("Text: \"%1\". Congratulations!")
          .arg((&text, AsStruct))));
      }),
      line_edit_edited: SlotNoArgs::new(move || {
        uref(button1).set_enabled(!uref(line_edit1).text(AsStruct).is_empty());
      }),
    };
    uref(button).signals().clicked().connect(&form.button_clicked);
    uref(line_edit).signals().text_edited().connect(&form.line_edit_edited);
    form
  }
}

fn main() {
  Application::create_and_exit(|_| {
    let _form = Form::new();
    Application::exec()
  })
}
