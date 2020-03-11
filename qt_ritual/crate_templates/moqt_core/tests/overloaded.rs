use moqt_core::{Overloaded, QPoint, QString};

#[test]
fn overloaded() {
    unsafe {
        Overloaded::from_float(1.0);
        Overloaded::from_q_string(QString::from_std_str("text1").as_ref());
        let a = Overloaded::from_int(1);

        a.at(6);
        a.at_mut(6);

        a.set_pos_2_int(3, 4);
        a.set_pos_q_point_bool(QPoint::new_0a().as_ref(), false);

        a.match_0a();
        a.match_1a(42);
    }
}
