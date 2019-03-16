use qt_core::Signal;
use qt_core::QCoreApplication;
use qt_core::SlotVariantRef;
use qt_core::QVariant;
use qt_core::QVariantAnimation;

#[test]
fn variant_animation2() {
    QCoreApplication::create_and_exit(|app| {
        let slot1 = SlotVariantRef::new(|value| {
            println!("value_changed: {}", value.to_string().to_std_string());
        });

        let mut animation = QVariantAnimation::new();
        animation.value_changed().connect(&slot1);
        animation.finished().connect(&app.slots().quit());
        animation.set_start_value(&QVariant::new0(1));
        animation.set_end_value(&QVariant::new0(5));
        animation.set_duration(5000);
        animation.start();
        QCoreApplication::exec()
    })
}
