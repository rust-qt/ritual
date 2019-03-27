use cpp_utils::CppBox;
use moqt_core::QPoint;

#[test]
fn create() {
    unsafe {
        let point: CppBox<QPoint> = QPoint::new_0a();
        assert_eq!(point.x(), 0);
        assert_eq!(point.y(), 0);
    }
}

#[test]
fn modify() {
    unsafe {
        let mut point: CppBox<QPoint> = QPoint::new_2a(2, 3);
        assert_eq!(point.x(), 2);
        assert_eq!(point.y(), 3);
        point.set_x(4);
        assert_eq!(point.x(), 4);
        point.set_y(-5);
        assert_eq!(point.y(), -5);
        assert_eq!(point.x(), 4);
    }
}

#[test]
fn vec() {
    unsafe {
        let mut vec: Vec<CppBox<QPoint>> = (0..20).map(|y| QPoint::new_2a(1, y)).collect();
        assert_eq!(vec.len(), 20);
        assert_eq!(vec[5].x(), 1);
        assert_eq!(vec[5].y(), 5);

        assert_eq!(vec[7].x(), 1);
        assert_eq!(vec[7].y(), 7);

        vec.remove(0);
        assert_eq!(vec[7].x(), 1);
        assert_eq!(vec[7].y(), 8);
    }
}
