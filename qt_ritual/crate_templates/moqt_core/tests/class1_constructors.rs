use moqt_core::BasicClass;

#[test]
fn class1_constructor1() {
    unsafe {
        let mut v = BasicClass::new(1);
        assert_eq!(v.foo(), 1);
        v.set_foo(5);
        assert_eq!(v.foo(), 5);
        assert_eq!(v.public_int_field(), 1);
        v.set_public_int_field(3);
        assert_eq!(v.public_int_field(), 3);
    }
}
