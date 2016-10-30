extern crate rust_ctrt1;
use rust_ctrt1::cpp_utils::{AsStruct, AsBox, CppBox};
use rust_ctrt1::class1::Class1;

#[test]
fn class1_constructor1() {
  let mut v: Class1 = Class1::new((1, AsStruct));
  assert_eq!(v.x(), 1);
}

#[test]
fn class1_constructor2() {
  let mut v: CppBox<Class1> = Class1::new((1, AsBox));
  assert_eq!(v.x(), 1);
}
