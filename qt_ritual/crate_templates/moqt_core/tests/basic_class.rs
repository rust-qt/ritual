use moqt_core::BasicClass;

#[test]
fn basic_class() {
    unsafe {
        let mut v = BasicClass::new(1);
        assert_eq!(v.foo(), 1);
        v.set_foo(5);
        assert_eq!(v.foo(), 5);

        assert_eq!(v.int_field(), 1);
        v.set_int_field(3);
        assert_eq!(v.int_field(), 3);
    }
}
