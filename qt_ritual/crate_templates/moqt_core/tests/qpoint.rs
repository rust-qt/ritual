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

#[test]
fn operators() {
    unsafe {
        let a: CppBox<QPoint> = QPoint::new_2a(1, 2);
        let b: CppBox<QPoint> = QPoint::new_2a(3, 4);
        let mut c: CppBox<QPoint> = &a + b.as_ref();
        assert_eq!(c.x(), 4);
        assert_eq!(c.y(), 6);

        c += a.as_ref();
        assert_eq!(c.x(), 5);
        assert_eq!(c.y(), 8);

        assert_eq!(c, QPoint::new_2a(5, 8).as_ref());
        assert!(c != QPoint::new_2a(5, 9).as_ref());
    }
}
