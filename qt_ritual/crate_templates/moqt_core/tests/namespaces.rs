use moqt_core::ns1;

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
