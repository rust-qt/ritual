use qt_gui::{qt_core::QPoint, QGuiApplication, QWindow};

#[test]
fn window1() {
    QGuiApplication::init(|_| unsafe {
        let mut a = QWindow::new();
        let mut b = QWindow::from_q_window(&mut a);
        let mut c = QWindow::from_q_window(&mut b);
        a.set_geometry_4a(10, 10, 300, 300);
        b.set_geometry_4a(20, 20, 200, 200);
        c.set_geometry_4a(40, 40, 100, 100);

        let point = QPoint::new_2a(100, 100);
        let r1 = a.map_to_global(&point);
        assert_eq!(r1.x(), 110);
        assert_eq!(r1.y(), 110);
        0
    })
}
