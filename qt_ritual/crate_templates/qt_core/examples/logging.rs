use qt_core::{
    q_critical, q_debug, q_info, q_set_message_pattern, q_warning, QCoreApplication, QString,
};

fn main() {
    QCoreApplication::init(|_app| unsafe {
        q_set_message_pattern(&QString::from_std_str(
            "%{file}:%{line} [%{function}] %{type}: %{message}",
        ));

        let _ = q_debug!() << QString::from_std_str("Example debug").as_ref();
        let _ = q_info!() << QString::from_std_str("Example info").as_ref() << 1i64;
        let _ = q_warning!() << QString::from_std_str("Example warning").as_ref();
        let _ = q_critical!() << QString::from_std_str("Example critical").as_ref();
        0
    })
}
