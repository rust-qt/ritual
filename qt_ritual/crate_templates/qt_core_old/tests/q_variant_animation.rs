use cpp_core::NullPtr;
use qt_core::{QCoreApplication, QVariant, QVariantAnimation, SlotOfQVariant};

#[test]
fn variant_animation2() {
    QCoreApplication::init(|app| unsafe {
        let mut next_value = 1;
        let slot1 = SlotOfQVariant::new(NullPtr, move |value| {
            assert_eq!(next_value, value.to_int_0a());
            assert_eq!(next_value.to_string(), value.to_string().to_std_string());
            next_value += 1;
        });

        let animation = QVariantAnimation::new_0a();
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
