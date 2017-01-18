
use ::term_painter::{Color, ToStyle};
use std::borrow::Borrow;
use std;

pub fn error<T: Borrow<str>>(text: T) {
  println!("{}", Color::Red.paint(text.borrow()));
}
pub fn warning<T: Borrow<str>>(text: T) {
  if std::env::var("CPP_TO_RUST_QUIET").is_err() {
    println!("{}", Color::Magenta.paint(text.borrow()));
  }
}
pub fn info<T: Borrow<str>>(text: T) {
  println!("{}", Color::Green.paint(text.borrow()));
}
pub fn debug<T: Borrow<str>>(text: T) {
  if std::env::var("CPP_TO_RUST_QUIET").is_err() {
    println!("{}", text.borrow());
  }
}

#[allow(unused_variables)]
pub fn noisy<T: Borrow<str>>(text: T) {}
