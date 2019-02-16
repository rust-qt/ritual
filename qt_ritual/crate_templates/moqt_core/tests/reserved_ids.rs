use moqt_core::{self_, Impl, Unsafe};

#[test]
fn reserved_ids() {
    let x: Impl = Impl::Trait;
    assert_eq!(x.to_int(), 0);
    assert_eq!(Impl::Use.to_int(), 1);
    assert_eq!(Impl::Crate.to_int(), 1);
    assert_eq!(Impl::Last.to_int(), -1);

    unsafe {
        let mut obj = Unsafe::new();
        assert_eq!(obj.loop_(), 1);
        obj.yield_(1);
        assert_eq!(obj.pub_().loop_(), 1);

        obj.set_super(2.4);
        assert_eq!(obj.super_(), 2.4);

        self_::box_(22);
    }
}
