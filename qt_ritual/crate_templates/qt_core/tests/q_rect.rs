#[test]
fn qrect() {
    unsafe {
        let r = qt_core::QRect::new4(1, 2, 3, 4);
        assert!(r.width() == 3);
    }
}
