extern crate rust_ctrt1;

#[test]
fn utils1() {
  use rust_ctrt1::utils::ctrt1_abs;
  assert_eq!(ctrt1_abs(1), 1);
  assert_eq!(ctrt1_abs(0), 0);
  assert_eq!(ctrt1_abs(-2), 2);
}
