extern crate ansi_term;
use self::ansi_term::Colour;
use std::borrow::Borrow;

pub fn error<T: Borrow<str>>(text: T) {
  println!("{}", Colour::Red.paint(text.borrow()));
}
pub fn warning<T: Borrow<str>>(text: T) {
  println!("{}", Colour::Purple.paint(text.borrow()));
}
pub fn info<T: Borrow<str>>(text: T) {
  println!("{}", Colour::Green.paint(text.borrow()));
}
pub fn debug<T: Borrow<str>>(text: T) {
  println!("{}", text.borrow());
}

#[allow(unused_variables)]
pub fn noisy<T: Borrow<str>>(text: T) {
}

