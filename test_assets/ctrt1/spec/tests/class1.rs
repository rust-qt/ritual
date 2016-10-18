extern crate rust_ctrt1;
use rust_ctrt1::cpp_utils::{RustManaged, CppPointer, CppBox};
use rust_ctrt1::class1::Class1;

#[test]
fn class1_1() {
  let mut v: Class1 = Class1::new((1, RustManaged));
  assert_eq!(v.x(), 1);
}

#[test]
fn class1_2() {
  let v: *mut Class1 = Class1::new((1, CppPointer));
  assert_eq!(unsafe { (*v).x() }, 1);
}
