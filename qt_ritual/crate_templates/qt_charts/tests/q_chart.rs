use qt_charts::{qt_core::QString, qt_widgets::QApplication, QChart};

#[test]
fn chart1() {
    QApplication::init(|_| unsafe {
        let mut chart = QChart::new_0a();
        chart.set_title(&QString::from_std_str("test"));
        let title = chart.title().to_std_string();
        assert_eq!(&title, "test");
        0
    })
}
