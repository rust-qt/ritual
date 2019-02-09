use moqt_core::{BaseClass1, DerivedClass1, DerivedClass2};
use cpp_utils::*;

#[test]
fn class1_constructor1() {
    unsafe {
        let mut derived = DerivedClass1::new();
        assert_eq!(derived.base_function(), 1);

        let mut base: Ptr<BaseClass1> = derived.static_upcast_mut();
        assert_eq!(base.base_function(), 2);

        let mut derived1: Ptr<DerivedClass1> = base.dynamic_cast_mut().unwrap();
        assert_eq!(derived1.base_function(), 3);

        let derived2: Option<Ptr<DerivedClass2>> = base.dynamic_cast_mut();
        assert!(derived2.is_none());

    }
}
