use qt_core::{QCoreApplication, QVariant, QVariantAnimation, SlotOfQVariantConstPtr};

#[test]
fn variant_animation2() {
    QCoreApplication::create_and_exit(|app| unsafe {
        let slot1 = SlotOfQVariantConstPtr::new(|value| {
            println!("value_changed: {}", value.to_string().to_std_string());
        });

        let mut animation = QVariantAnimation::new_0a();
        animation.value_changed().connect(&slot1);
        animation.finished().connect(app.slot_quit());
        animation.set_start_value(QVariant::new7(1).as_ptr());
        animation.set_end_value(QVariant::new7(5).as_ptr());
        animation.set_duration(5000);
        animation.start_0a();
        QCoreApplication::exec()
    })
}
