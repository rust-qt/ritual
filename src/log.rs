extern crate ansi_term;
use self::ansi_term::Colour;
use std::borrow::Borrow;
use std;
use std::io::Write;

pub fn error<T: Borrow<str>>(text: T) {
  writeln!(&mut std::io::stderr(),
           "{}",
           Colour::Red.paint(text.borrow()))
    .unwrap();
}
pub fn warning<T: Borrow<str>>(text: T) {
  writeln!(&mut std::io::stderr(),
           "{}",
           Colour::Purple.paint(text.borrow()))
    .unwrap();
}
pub fn info<T: Borrow<str>>(text: T) {
  writeln!(&mut std::io::stderr(),
           "{}",
           Colour::Green.paint(text.borrow()))
    .unwrap();
}
pub fn debug<T: Borrow<str>>(text: T) {
  writeln!(&mut std::io::stderr(), "{}", text.borrow()).unwrap();
}

#[allow(unused_variables)]
pub fn noisy<T: Borrow<str>>(text: T) {}
