use qt_widgets::{qt_core::QString, QApplication, QPushButton};

#[test]
fn push_button1() {
    QApplication::init(|_| unsafe {
        let btn = QPushButton::from_q_string(&QString::from_std_str("first_button"));
        let text = btn.text().to_std_string();
        assert_eq!(&text, "first_button");
        0
    })
}
