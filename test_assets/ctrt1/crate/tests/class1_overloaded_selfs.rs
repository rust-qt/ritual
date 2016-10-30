extern crate rust_ctrt1;
use rust_ctrt1::cpp_utils::{AsStruct, AsBox, CppBox};
use rust_ctrt1::class1::Class1;

#[test]
fn class1_overloaded_selfs() {
  let mut v = Class1::new((1, AsStruct));
  v.f1();
  v.f1_mut();
  Class1::f1_static(0);

  v.f2();
  v.f2_mut();

  v.f3();
  Class1::f3_static(0);

  v.f4();
  Class1::f4_static(0);
}
