extern crate rust_ctrt1;
use rust_ctrt1::cpp_utils::{AsStruct, AsBox, CppBox};
use rust_ctrt1::class1::Class1;

#[test]
fn class1_overloaded_selfs() {
  let mut v = Class1::new((1, AsStruct));
  v.f1_from_const();
  v.f1_from_mut();
  Class1::f1_static(0);

  v.f2_from_const();
  v.f2_from_mut();

  v.f3_from_const();
  Class1::f3_static(0);

  v.f4_from_mut();
  Class1::f4_static(0);
}
