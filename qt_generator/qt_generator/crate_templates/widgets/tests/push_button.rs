extern crate qt_widgets;

use qt_widgets::application::Application;
use qt_widgets::push_button::PushButton;
use qt_widgets::cpp_utils::*;
use qt_widgets::qt_core::string::String;
use qt_widgets::libc::{c_char, c_int};

#[test]
fn push_button1() {
  Application::create_and_exit(|_| {
    let btn = PushButton::new((&String::from_std_str("first_button"), AsBox));
    let text = btn.text(AsStruct).to_std_string();
    assert_eq!(&text, "first_button");
    0
  })
}
