#[test]
fn qrect() {
    unsafe {
        let r = qt_core::QRect::from_4_int(1, 2, 3, 4);
        assert!(r.width() == 3);
    }
}
