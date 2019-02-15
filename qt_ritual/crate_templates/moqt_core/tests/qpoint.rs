use moqt_core::QPoint;

#[test]
fn create() {
    unsafe {
        let point: QPoint = QPoint::new();
        assert_eq!(point.x(), 0);
        assert_eq!(point.y(), 0);
    }
}

#[test]
fn modify() {
    unsafe {
        let mut point: QPoint = QPoint::new2(2, 3);
        assert_eq!(point.x(), 2);
        assert_eq!(point.y(), 3);
        point.set_x(4);
        assert_eq!(point.x(), 4);
        point.set_y(-5);
        assert_eq!(point.y(), -5);
        assert_eq!(point.x(), 4);
    }
}
