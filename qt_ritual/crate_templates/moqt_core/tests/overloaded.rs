use moqt_core::{Overloaded, QPoint, QString};

#[test]
fn overloaded() {
    unsafe {
        Overloaded::new_from_float(1.0);
        Overloaded::new_from_q_string(QString::from_std_str("text1").as_ptr());
        let mut a = Overloaded::new_from_int(1);

        a.at(6);
        a.at_mut(6);

        a.set_pos_from_x_y(3, 4);
        a.set_pos_from_pos_flag(QPoint::new_0a().as_ptr(), false);
    }
}
