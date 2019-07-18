use qt_widgets::{qt_core::QString, QApplication, QPushButton};

#[test]
fn push_button1() {
    QApplication::create_and_exit(|_| unsafe {
        let btn = QPushButton::new5(QString::from_std_str("first_button").as_ref());
        let text = btn.text().to_std_string();
        assert_eq!(&text, "first_button");
        0
    })
}
