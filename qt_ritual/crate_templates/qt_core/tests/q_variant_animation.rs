use qt_core::{QCoreApplication, QVariant, QVariantAnimation, SlotOfQVariant};

#[test]
fn variant_animation2() {
    QCoreApplication::init(|app| unsafe {
        let slot1 = SlotOfQVariant::new(|value| {
            println!("value_changed: {}", value.to_string().to_std_string());
        });

        let mut animation = QVariantAnimation::new_0a();
        let c = animation.value_changed().connect(&slot1);
        assert!(c.is_valid());
        let c = animation.finished().connect(app.slot_quit());
        assert!(c.is_valid());
        animation.set_start_value(&QVariant::from_int(1));
        animation.set_end_value(&QVariant::from_int(5));
        animation.set_duration(5000);
        animation.start_0a();
        QCoreApplication::exec()
    })
}
