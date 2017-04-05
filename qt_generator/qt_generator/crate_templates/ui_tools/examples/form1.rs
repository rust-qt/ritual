extern crate qt_ui_tools;

use qt_ui_tools::ui_loader::UiLoader;
use qt_ui_tools::qt_widgets::widget::Widget;
use qt_ui_tools::qt_core::file::File;
use qt_ui_tools::qt_core::flags::Flags;
use qt_ui_tools::qt_core::io_device::OpenModeFlag;

use qt_ui_tools::cpp_utils::{CppBox, StaticCast, static_cast_mut};
use qt_ui_tools::qt_widgets::application::Application;
use qt_ui_tools::qt_core::connections::Signal;

use qt_ui_tools::qt_core::string::String;
use qt_ui_tools::qt_core::slots::SlotNoArgs;

use std::cell::RefCell;

struct Form<'a> {
  widget: CppBox<Widget>,
  check_box_toggled: SlotNoArgs<'a>,
}

fn uref<T>(ptr: *mut T) -> &'static mut T {
  unsafe { ptr.as_mut() }.expect("null pointer in uref")
}

impl<'a> Form<'a> {
  fn new() -> Form<'a> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/form1.ui");
    let mut file = File::new(&String::from_std_str(path));
    assert!(file.open(Flags::from_enum(OpenModeFlag::ReadOnly)));

    let mut ui_loader = UiLoader::new(());
    // TODO: UiLoader::load should return CppBox<Widget> instead of *mut Widget
    let widget_ptr = ui_loader.load(static_cast_mut(file.as_mut_ptr()));
    let mut widget = unsafe { CppBox::new(widget_ptr) };
    widget.show();

    let form = Form {
      widget: widget,
      check_box_toggled: SlotNoArgs::new(move || {}),
    };
    //uref(button).signals().clicked().connect(&form.button_clicked);
    //uref(line_edit).signals().text_edited().connect(&form.line_edit_edited);
    form
  }
}

fn main() {
  Application::create_and_exit(|_| {
                                 let _form = Form::new();
                                 Application::exec()
                               })
}
