extern crate qt_widgets;
use qt_widgets::cpp_utils::{CppBox, StaticCast};
use qt_widgets::application::Application;
use qt_widgets::qt_core::connections::Signal;

use qt_widgets::widget::Widget;
use qt_widgets::push_button::PushButton;
use qt_widgets::line_edit::LineEdit;
use qt_widgets::v_box_layout::VBoxLayout;
use qt_widgets::qt_core::string::String;
use qt_widgets::qt_core::slots::SlotNoArgs;
use qt_widgets::message_box::MessageBox;

struct Form<'a> {
  _widget: CppBox<Widget>,
  _button: *mut PushButton,
  _line_edit: *mut LineEdit,
  button_clicked: SlotNoArgs<'a>,
  line_edit_edited: SlotNoArgs<'a>,
}

fn uref<T>(ptr: *mut T) -> &'static mut T {
  unsafe { ptr.as_mut() }.expect("null pointer in uref")
}

impl<'a> Form<'a> {
  fn new() -> Form<'a> {
    let mut widget = Widget::new();
    let mut layout = unsafe { VBoxLayout::new_unsafe(widget.as_mut_ptr()) };
    let mut line_edit = LineEdit::new(());
    unsafe {
      layout.add_widget(line_edit.static_cast_mut() as *mut _);
    }
    let line_edit = line_edit.into_raw();
    let mut button = PushButton::new(&String::from_std_str("Start"));
    button.set_enabled(false);
    unsafe {
      layout.add_widget(button.static_cast_mut() as *mut _);
    }
    let button = button.into_raw();
    layout.into_raw();
    widget.show();

    let button1 = button;
    let line_edit1 = line_edit;
    let widget1 = widget.as_mut_ptr();

    let form = Form {
      _widget: widget,
      _button: button,
      _line_edit: line_edit,
      button_clicked: SlotNoArgs::new(move || {
        let text = uref(line_edit1).text();
        unsafe {
          MessageBox::information((widget1,
                                   &String::from_std_str("My title"),
                                   &String::from_std_str("Text: \"%1\". Congratulations!")
                                      .arg0(&text)));
        }
      }),
      line_edit_edited: SlotNoArgs::new(move || {
                                          uref(button1).set_enabled(!uref(line_edit1)
                                                                       .text()
                                                                       .is_empty());
                                        }),
    };
    uref(button)
      .signals()
      .clicked()
      .connect(&form.button_clicked);
    uref(line_edit)
      .signals()
      .text_edited()
      .connect(&form.line_edit_edited);
    form
  }
}

fn main() {
  Application::create_and_exit(|_| {
                                 let _form = Form::new();
                                 Application::exec()
                               })
}
