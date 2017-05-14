extern crate rust_ctrt1;
use rust_ctrt1::class1::Class1;

#[test]
fn class1_constructor1() {
  let mut v: Class1 = Class1::new(1);
  assert_eq!(v.x(), 1);
  assert_eq!(v.field1(), 1);
  v.set_field1(3);
  assert_eq!(v.field1(), 3);

}
