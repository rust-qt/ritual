use cpp_core::CppBox;
use moqt_core::{ns1, Class3, Templated3OfInt};

#[test]
fn namespaces() {
    unsafe {
        assert_eq!(ns1::x(), 1);
        assert_eq!(ns1::ns2::x(), 2);
        assert_eq!(ns1::ns2::y(), 3);
        assert_eq!(ns1::ns2::Enum1::Val1.to_int(), 0);
        assert_eq!(ns1::ns2::Enum1::Val2.to_int(), 1);
        assert_eq!(ns1::ns2::Enum1::Val3.to_int(), 2);
        assert_eq!(ns1::ns3::a(), 4);
        assert_eq!(ns1::ns3::b(), 5);
        assert_eq!(ns1::ns3::Enum2::Val11.to_int(), 1);
        assert_eq!(ns1::ns3::Enum2::Val12.to_int(), 2);
        assert_eq!(ns1::ns3::Enum2::Val13.to_int(), 3);

        ns1::ns3::ns4::Class1::new(1);
    }
}

#[test]
fn namespaces2() {
    unsafe {
        let mut x: CppBox<ns1::Templated1OfInt> = moqt_core::func1();
        x.x();

        let mut y: CppBox<ns1::class_ns::Templated2OfBool> = moqt_core::func2();
        y.y();

        let _z = ns1::class_ns::Class1::new();
        let _t = ns1::ClassNs::new();
    }
}

#[test]
fn ignored_namespace() {
    unsafe {
        let _a = Class3::new();
        let _b: CppBox<Templated3OfInt> = moqt_core::func3();
    }
}
