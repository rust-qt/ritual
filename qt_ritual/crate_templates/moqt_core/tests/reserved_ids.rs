use moqt_core::{impl_, unsafe_, self_};

#[test]
fn reserved_ids() {
    let x: impl_ = impl_::trait_;
    assert_eq!(x.to_int(), 0);
    assert_eq!(impl_::use_.to_int(), 1);
    assert_eq!(impl_::crate_.to_int(), 1);
    assert_eq!(impl_::last.to_int(), -1);

    unsafe {
        let mut obj = unsafe_::new();
        assert_eq!(obj.loop_(), 1);
        obj.yield_(1);
        assert_eq!(obj.pub_().loop_(), 1);

        obj.set_super(2.4);
        assert_eq!(obj.super_(), 2.4);

        self_::box_(22);
    }
}
