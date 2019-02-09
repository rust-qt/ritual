use moqt_core::{BaseClass1, DerivedClass1, DerivedClass2};
use cpp_utils::*;

#[test]
fn casts() {
    unsafe {
        let mut derived = DerivedClass1::new();
        assert_eq!(derived.base_function(), 1);

        let mut base: Ptr<BaseClass1> = derived.static_upcast_mut();
        assert_eq!(base.base_function(), 2);

        let base_const: ConstPtr<BaseClass1> = derived.static_upcast();
        assert_eq!(base_const.base_const_function(), 2);

        let mut derived1: Ptr<DerivedClass1> = base.dynamic_cast_mut().unwrap();
        assert_eq!(derived1.base_function(), 3);

        let derived1: ConstPtr<DerivedClass1> = base.dynamic_cast().unwrap();
        assert_eq!(derived1.base_const_function(), 3);

        let derived2: Option<Ptr<DerivedClass2>> = base.dynamic_cast_mut();
        assert!(derived2.is_none());

        let mut derived1: Ptr<DerivedClass1> = base.static_downcast_mut();
        assert_eq!(derived1.base_function(), 4);

        let derived1: ConstPtr<DerivedClass1> = base.static_downcast();
        assert_eq!(derived1.base_const_function(), 4);
    }
}

#[test]
fn virtual_functions() {
    unsafe {
        let derived = DerivedClass1::new();
        assert_eq!(derived.virtual_function(), 43);

        let base: ConstPtr<BaseClass1> = derived.static_upcast();
        assert_eq!(base.virtual_function(), 43);

        let base = BaseClass1::new();
        assert_eq!(base.virtual_function(), 42);

        let derived = DerivedClass2::new();
        assert_eq!(derived.virtual_function(), 44);

        let base: ConstPtr<BaseClass1> = derived.static_upcast();
        assert_eq!(base.virtual_function(), 44);
    }
}